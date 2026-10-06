use pc_core::{
    DuplicateDeleteResult, DuplicateFile, DuplicateGroup, DuplicatePlanItem, DuplicateScanOptions,
    DuplicateScanPlan, DuplicateScanSummary, FilesystemError, FilesystemWarning, StorageFileEntry,
    StorageFolderAggregate, StorageScanOptions, StorageScanSummary, StorageTypeAggregate,
};
#[cfg(target_os = "windows")]
use pc_core::DuplicateDeleteError;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
use std::fs::{self, File};
#[cfg(target_os = "windows")]
use std::io::Write;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::process::Command;

const PARTIAL_HASH_BYTES: usize = 64 * 1024;
const HASH_BUFFER_BYTES: usize = 128 * 1024;
const MAX_WARNINGS: usize = 100;
#[cfg(target_os = "windows")]
const DUPLICATE_AUDIT_FILE: &str = "duplicates.jsonl";

#[derive(Debug, Clone)]
struct FileRecord {
    root: PathBuf,
    path: PathBuf,
    bytes: u64,
    modified_at_epoch_ms: Option<u64>,
}

pub fn scan_root_id(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-scan-root-v1:");
    hasher.update(normalized_path(path).as_bytes());
    let digest = hasher.finalize();
    format!(
        "root-{}",
        digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn select_user_scan_root() -> Result<Option<PathBuf>, FilesystemError> {
    #[cfg(target_os = "windows")]
    {
        const PICKER_SCRIPT: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Select a folder for PC Manager to scan'
$dialog.ShowNewFolderButton = $false
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
  Write-Output $dialog.SelectedPath
}
"#;

        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                PICKER_SCRIPT,
            ])
            .output()
            .map_err(|error| {
                FilesystemError::new(
                    "folder_picker_unavailable",
                    format!("Unable to open the Windows folder picker: {error}"),
                    true,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FilesystemError::new(
                "folder_picker_failed",
                if stderr.trim().is_empty() {
                    "The Windows folder picker failed.".to_string()
                } else {
                    format!("The Windows folder picker failed: {}", stderr.trim())
                },
                true,
            ));
        }

        let selected = String::from_utf8(output.stdout).map_err(|error| {
            FilesystemError::new(
                "folder_picker_encoding",
                format!("The Windows folder picker returned invalid text: {error}"),
                true,
            )
        })?;
        let selected = selected.trim();
        if selected.is_empty() {
            return Ok(None);
        }

        let path = fs::canonicalize(selected).map_err(|error| {
            FilesystemError::new(
                "selected_root_unavailable",
                format!("The selected folder cannot be resolved: {error}"),
                true,
            )
        })?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            FilesystemError::new(
                "selected_root_unavailable",
                format!("The selected folder cannot be inspected: {error}"),
                true,
            )
        })?;
        if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
            return Err(FilesystemError::new(
                "selected_root_unsafe",
                "The selected scan root must be an ordinary directory, not a link or reparse point.",
                false,
            ));
        }

        Ok(Some(path))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(FilesystemError::new(
            "unsupported_platform",
            "Folder selection is currently implemented for Windows only.",
            false,
        ))
    }
}

pub fn scan_duplicate_files(
    roots: &[PathBuf],
    options: DuplicateScanOptions,
    cancel: &AtomicBool,
) -> Result<(DuplicateScanPlan, DuplicateScanSummary), FilesystemError> {
    let excluded = normalized_exclusions(&options.excluded_directory_names);
    let min_file_bytes = options.min_file_bytes.max(1);
    let max_groups = options.max_groups.clamp(1, 1_000);
    let max_files_per_group = options.max_files_per_group.clamp(2, 200);
    let (files, mut warnings) = walk_regular_files(roots, &excluded, min_file_bytes, cancel)?;

    ensure_not_cancelled(cancel)?;

    let scanned_files = files.len() as u64;
    let scanned_bytes = files.iter().map(|file| file.bytes).sum();

    let mut by_size: BTreeMap<u64, Vec<FileRecord>> = BTreeMap::new();
    for file in files {
        by_size.entry(file.bytes).or_default().push(file);
    }

    let mut by_partial: HashMap<(u64, String), Vec<FileRecord>> = HashMap::new();
    for (size, group) in by_size.into_iter().filter(|(_, files)| files.len() > 1) {
        for file in group {
            ensure_not_cancelled(cancel)?;
            match partial_hash(&file.path, size) {
                Ok(hash) => by_partial.entry((size, hash)).or_default().push(file),
                Err(error) => push_warning(
                    &mut warnings,
                    file.path.to_string_lossy().to_string(),
                    format!("Partial hash skipped: {error}"),
                ),
            }
        }
    }

    let mut by_full: HashMap<(u64, String), Vec<FileRecord>> = HashMap::new();
    for ((size, _partial), group) in by_partial.into_iter().filter(|(_, files)| files.len() > 1) {
        for file in group {
            ensure_not_cancelled(cancel)?;
            match full_hash(&file.path) {
                Ok(hash) => by_full.entry((size, hash)).or_default().push(file),
                Err(error) => push_warning(
                    &mut warnings,
                    file.path.to_string_lossy().to_string(),
                    format!("Full hash skipped: {error}"),
                ),
            }
        }
    }

    let scan_id = format!("duplicate-scan-{}-{}", now_epoch_ms(), std::process::id());
    let mut groups = Vec::new();
    let mut plan_items = Vec::new();

    let mut verified = by_full
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect::<Vec<_>>();
    verified.sort_by(|((size_a, _), files_a), ((size_b, _), files_b)| {
        let waste_a = size_a.saturating_mul(files_a.len().saturating_sub(1) as u64);
        let waste_b = size_b.saturating_mul(files_b.len().saturating_sub(1) as u64);
        waste_b.cmp(&waste_a)
    });

    for ((size, full_hash_value), files) in verified.into_iter().take(max_groups) {
        ensure_not_cancelled(cancel)?;
        let group_id = format!("dup-{}-{size}", &full_hash_value[..16]);
        let original_count = files.len();
        let shown = files
            .into_iter()
            .take(max_files_per_group)
            .collect::<Vec<_>>();

        if original_count > shown.len() {
            push_warning(
                &mut warnings,
                group_id.clone(),
                format!(
                    "This duplicate group contains {original_count} files; only the first {} are shown.",
                    shown.len()
                ),
            );
        }

        let mut public_files = Vec::with_capacity(shown.len());
        for file in shown {
            let id = duplicate_file_id(&file.path, file.bytes, file.modified_at_epoch_ms);
            public_files.push(DuplicateFile {
                id: id.clone(),
                path: file.path.to_string_lossy().to_string(),
                bytes: file.bytes,
                modified_at_epoch_ms: file.modified_at_epoch_ms,
            });
            plan_items.push(DuplicatePlanItem {
                id,
                group_id: group_id.clone(),
                root: file.root.to_string_lossy().to_string(),
                path: file.path.to_string_lossy().to_string(),
                bytes: file.bytes,
                modified_at_epoch_ms: file.modified_at_epoch_ms,
                full_hash: full_hash_value.clone(),
            });
        }

        if public_files.len() > 1 {
            let total_bytes = size.saturating_mul(public_files.len() as u64);
            let recoverable_bytes =
                size.saturating_mul(public_files.len().saturating_sub(1) as u64);
            groups.push(DuplicateGroup {
                id: group_id,
                bytes_each: size,
                total_bytes,
                recoverable_bytes,
                files: public_files,
            });
        }
    }

    let plan = DuplicateScanPlan {
        scan_id: scan_id.clone(),
        items: plan_items,
    };
    let summary = DuplicateScanSummary {
        scan_id,
        scanned_files,
        scanned_bytes,
        duplicate_groups: groups,
        warnings,
    };

    Ok((plan, summary))
}

pub fn scan_storage(
    roots: &[PathBuf],
    options: StorageScanOptions,
    cancel: &AtomicBool,
) -> Result<StorageScanSummary, FilesystemError> {
    let excluded = normalized_exclusions(&options.excluded_directory_names);
    let top_files = options.top_files.clamp(10, 200);
    let top_folders = options.top_folders.clamp(10, 200);
    let (files, warnings) = walk_regular_files(roots, &excluded, 0, cancel)?;
    ensure_not_cancelled(cancel)?;

    let total_files = files.len() as u64;
    let total_bytes = files.iter().map(|file| file.bytes).sum();

    let mut folder_totals: HashMap<String, (u64, u64)> = HashMap::new();
    let mut type_totals: HashMap<String, (u64, u64)> = HashMap::new();

    for file in &files {
        ensure_not_cancelled(cancel)?;
        let extension = file
            .path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{}", value.to_ascii_lowercase()))
            .unwrap_or_else(|| "[no extension]".to_string());
        let type_entry = type_totals.entry(extension).or_default();
        type_entry.0 = type_entry.0.saturating_add(file.bytes);
        type_entry.1 = type_entry.1.saturating_add(1);

        let mut current = file.path.parent();
        while let Some(folder) = current {
            if !folder.starts_with(&file.root) {
                break;
            }
            let key = folder.to_string_lossy().to_string();
            let aggregate = folder_totals.entry(key).or_default();
            aggregate.0 = aggregate.0.saturating_add(file.bytes);
            aggregate.1 = aggregate.1.saturating_add(1);

            if folder == file.root {
                break;
            }
            current = folder.parent();
        }
    }

    let mut largest_files = files
        .iter()
        .map(|file| StorageFileEntry {
            path: file.path.to_string_lossy().to_string(),
            bytes: file.bytes,
        })
        .collect::<Vec<_>>();
    largest_files.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    largest_files.truncate(top_files);

    let mut largest_folders = folder_totals
        .into_iter()
        .map(|(path, (bytes, file_count))| StorageFolderAggregate {
            path,
            bytes,
            file_count,
        })
        .collect::<Vec<_>>();
    largest_folders.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    largest_folders.truncate(top_folders);

    let mut file_types = type_totals
        .into_iter()
        .map(|(extension, (bytes, file_count))| StorageTypeAggregate {
            extension,
            bytes,
            file_count,
        })
        .collect::<Vec<_>>();
    file_types.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| a.extension.cmp(&b.extension))
    });
    file_types.truncate(100);

    Ok(StorageScanSummary {
        scan_id: format!("storage-scan-{}-{}", now_epoch_ms(), std::process::id()),
        total_files,
        total_bytes,
        largest_files,
        largest_folders,
        file_types,
        warnings,
    })
}

pub fn delete_duplicate_files(
    plan: &DuplicateScanPlan,
    requested_file_ids: &[String],
) -> Result<DuplicateDeleteResult, FilesystemError> {
    #[cfg(target_os = "windows")]
    {
        if requested_file_ids.is_empty() {
            return Err(FilesystemError::new(
                "duplicate_selection_empty",
                "Select at least one verified duplicate file to delete.",
                true,
            ));
        }

        let requested = requested_file_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let matched = plan
            .items
            .iter()
            .filter(|item| requested.contains(item.id.as_str()))
            .collect::<Vec<_>>();

        if matched.len() != requested.len() {
            return Err(FilesystemError::new(
                "duplicate_selection_stale",
                "One or more selected duplicate IDs are no longer part of the native scan plan. Scan again.",
                true,
            ));
        }

        let mut total_by_group: HashMap<&str, usize> = HashMap::new();
        let mut selected_by_group: HashMap<&str, usize> = HashMap::new();
        for item in &plan.items {
            *total_by_group.entry(item.group_id.as_str()).or_default() += 1;
        }
        for item in &matched {
            *selected_by_group.entry(item.group_id.as_str()).or_default() += 1;
        }
        if selected_by_group.iter().any(|(group_id, selected)| {
            total_by_group
                .get(group_id)
                .is_some_and(|total| selected >= total)
        }) {
            return Err(FilesystemError::new(
                "duplicate_keep_one_required",
                "PC Manager will not delete every verified copy in a duplicate group. Keep at least one copy.",
                false,
            ));
        }

        ensure_duplicate_operation_log_writable()?;

        let mut deleted_files = 0_u64;
        let mut deleted_bytes = 0_u64;
        let mut errors = Vec::new();

        for item in matched {
            match validate_and_delete_duplicate(item) {
                Ok(()) => {
                    deleted_files = deleted_files.saturating_add(1);
                    deleted_bytes = deleted_bytes.saturating_add(item.bytes);
                }
                Err((code, message)) => errors.push(DuplicateDeleteError {
                    file_id: item.id.clone(),
                    code,
                    message,
                }),
            }
        }

        let completed_at_epoch_ms = now_epoch_ms();
        let result = DuplicateDeleteResult {
            operation_id: format!(
                "duplicate-delete-{}-{}",
                completed_at_epoch_ms,
                std::process::id()
            ),
            scan_id: plan.scan_id.clone(),
            requested_files: requested.len() as u64,
            deleted_files,
            deleted_bytes,
            failed_files: errors.len() as u64,
            errors,
            completed_at_epoch_ms,
        };
        append_duplicate_operation(&result)?;
        Ok(result)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (plan, requested_file_ids);
        Err(FilesystemError::new(
            "unsupported_platform",
            "Duplicate deletion is currently implemented for Windows only.",
            false,
        ))
    }
}

fn walk_regular_files(
    roots: &[PathBuf],
    excluded_names: &HashSet<String>,
    min_file_bytes: u64,
    cancel: &AtomicBool,
) -> Result<(Vec<FileRecord>, Vec<FilesystemWarning>), FilesystemError> {
    if roots.is_empty() {
        return Err(FilesystemError::new(
            "scan_roots_empty",
            "Select at least one folder before scanning.",
            true,
        ));
    }

    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut canonical_roots = Vec::new();

    for root in roots {
        ensure_not_cancelled(cancel)?;
        let canonical = fs::canonicalize(root).map_err(|error| {
            FilesystemError::new(
                "scan_root_unavailable",
                format!("A selected scan root cannot be resolved: {error}"),
                true,
            )
        })?;
        let metadata = fs::symlink_metadata(&canonical).map_err(|error| {
            FilesystemError::new(
                "scan_root_unavailable",
                format!("A selected scan root cannot be inspected: {error}"),
                true,
            )
        })?;
        if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
            return Err(FilesystemError::new(
                "scan_root_unsafe",
                "A selected scan root is not an ordinary directory.",
                false,
            ));
        }
        if !canonical_roots.contains(&canonical) {
            canonical_roots.push(canonical);
        }
    }

    for root in canonical_roots {
        let mut queue = VecDeque::from([root.clone()]);

        while let Some(directory) = queue.pop_front() {
            ensure_not_cancelled(cancel)?;
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    push_warning(
                        &mut warnings,
                        directory.to_string_lossy().to_string(),
                        format!("Folder could not be read: {error}"),
                    );
                    continue;
                }
            };

            for entry in entries {
                ensure_not_cancelled(cancel)?;
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        push_warning(
                            &mut warnings,
                            directory.to_string_lossy().to_string(),
                            format!("Directory entry could not be read: {error}"),
                        );
                        continue;
                    }
                };
                let path = entry.path();
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        push_warning(
                            &mut warnings,
                            path.to_string_lossy().to_string(),
                            format!("Item could not be inspected: {error}"),
                        );
                        continue;
                    }
                };

                if is_reparse_or_symlink(&metadata) {
                    push_warning(
                        &mut warnings,
                        path.to_string_lossy().to_string(),
                        "Link or Windows reparse point skipped.".to_string(),
                    );
                    continue;
                }

                if metadata.is_dir() {
                    if should_exclude_directory(&path, excluded_names) {
                        continue;
                    }
                    queue.push_back(path);
                    continue;
                }

                if !metadata.is_file() || metadata.len() < min_file_bytes {
                    continue;
                }

                files.push(FileRecord {
                    root: root.clone(),
                    path,
                    bytes: metadata.len(),
                    modified_at_epoch_ms: metadata.modified().ok().and_then(epoch_ms),
                });
            }
        }
    }

    Ok((files, warnings))
}

fn normalized_exclusions(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_ascii_lowercase())
        })
        .collect()
}

fn should_exclude_directory(path: &Path, user_exclusions: &HashSet<String>) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    user_exclusions.contains(&name)
        || matches!(
            name.as_str(),
            "$recycle.bin"
                | "system volume information"
                | "windows"
                | "program files"
                | "program files (x86)"
                | "programdata"
        )
}

fn partial_hash(path: &Path, size: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    hasher.update(size.to_le_bytes());

    let first_len =
        usize::try_from(size.min(PARTIAL_HASH_BYTES as u64)).unwrap_or(PARTIAL_HASH_BYTES);
    let mut first = vec![0_u8; first_len];
    file.read_exact(&mut first)?;
    hasher.update(&first);

    if size > PARTIAL_HASH_BYTES as u64 {
        let tail_len =
            usize::try_from(size.min(PARTIAL_HASH_BYTES as u64)).unwrap_or(PARTIAL_HASH_BYTES);
        file.seek(SeekFrom::End(-(tail_len as i64)))?;
        let mut tail = vec![0_u8; tail_len];
        file.read_exact(&mut tail)?;
        hasher.update(&tail);
    }

    Ok(hex_digest(hasher.finalize()))
}

fn full_hash(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex_digest(hasher.finalize()))
}

fn duplicate_file_id(path: &Path, bytes: u64, modified_at_epoch_ms: Option<u64>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-duplicate-file-v1:");
    hasher.update(normalized_path(path).as_bytes());
    hasher.update(bytes.to_le_bytes());
    hasher.update(modified_at_epoch_ms.unwrap_or_default().to_le_bytes());
    let digest = hasher.finalize();
    format!(
        "file-{}",
        digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn push_warning(warnings: &mut Vec<FilesystemWarning>, source: String, message: String) {
    if warnings.len() < MAX_WARNINGS {
        warnings.push(FilesystemWarning { source, message });
    }
}

fn ensure_not_cancelled(cancel: &AtomicBool) -> Result<(), FilesystemError> {
    if cancel.load(Ordering::Relaxed) {
        Err(FilesystemError::new(
            "scan_cancelled",
            "The filesystem scan was cancelled.",
            true,
        ))
    } else {
        Ok(())
    }
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return true;
        }
    }

    false
}

#[cfg(target_os = "windows")]
fn validate_and_delete_duplicate(item: &DuplicatePlanItem) -> Result<(), (String, String)> {
    let root = fs::canonicalize(&item.root).map_err(|_| {
        (
            "duplicate_root_unavailable".to_string(),
            "The original duplicate scan root is no longer available.".to_string(),
        )
    })?;
    let path = PathBuf::from(&item.path);
    let metadata = fs::symlink_metadata(&path).map_err(|_| {
        (
            "duplicate_file_unavailable".to_string(),
            "The selected duplicate no longer exists or cannot be inspected.".to_string(),
        )
    })?;
    if !metadata.is_file() || is_reparse_or_symlink(&metadata) {
        return Err((
            "duplicate_file_unsafe".to_string(),
            "The selected duplicate is no longer an ordinary file.".to_string(),
        ));
    }

    let canonical = fs::canonicalize(&path).map_err(|_| {
        (
            "duplicate_file_unresolved".to_string(),
            "The selected duplicate could not be revalidated.".to_string(),
        )
    })?;
    if !canonical.starts_with(&root) {
        return Err((
            "duplicate_file_outside_root".to_string(),
            "The selected duplicate resolved outside its approved scan root.".to_string(),
        ));
    }
    if is_system_sensitive_path(&canonical) {
        return Err((
            "duplicate_system_path_rejected".to_string(),
            "PC Manager will not delete duplicate candidates inside protected Windows or program directories.".to_string(),
        ));
    }
    if metadata.len() != item.bytes
        || metadata.modified().ok().and_then(epoch_ms) != item.modified_at_epoch_ms
    {
        return Err((
            "duplicate_file_changed".to_string(),
            "The selected duplicate changed after the scan and was not deleted.".to_string(),
        ));
    }

    let current_hash = full_hash(&canonical).map_err(|error| {
        (
            "duplicate_rehash_failed".to_string(),
            format!("The selected duplicate could not be rehashed: {error}"),
        )
    })?;
    if current_hash != item.full_hash {
        return Err((
            "duplicate_hash_changed".to_string(),
            "The selected duplicate no longer matches the verified full hash.".to_string(),
        ));
    }

    fs::remove_file(&canonical).map_err(|error| {
        (
            "duplicate_delete_failed".to_string(),
            format!("Windows could not delete the selected duplicate: {error}"),
        )
    })
}

#[cfg(target_os = "windows")]
fn is_system_sensitive_path(path: &Path) -> bool {
    ["WINDIR", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"]
        .iter()
        .filter_map(|name| std::env::var_os(name))
        .map(PathBuf::from)
        .filter_map(|root| fs::canonicalize(root).ok())
        .any(|root| path.starts_with(root))
}

#[cfg(target_os = "windows")]
fn duplicate_operation_log_path() -> Result<PathBuf, FilesystemError> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        FilesystemError::new(
            "local_app_data_unavailable",
            "LOCALAPPDATA is unavailable, so duplicate deletion cannot be audited.",
            false,
        )
    })?;
    Ok(PathBuf::from(local)
        .join("PCManager")
        .join("operations")
        .join(DUPLICATE_AUDIT_FILE))
}

#[cfg(target_os = "windows")]
fn ensure_duplicate_operation_log_writable() -> Result<(), FilesystemError> {
    let path = duplicate_operation_log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            FilesystemError::new(
                "duplicate_audit_directory_failed",
                format!("Unable to prepare duplicate operation history: {error}"),
                true,
            )
        })?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| {
            FilesystemError::new(
                "duplicate_audit_open_failed",
                format!("Unable to prepare duplicate operation history: {error}"),
                true,
            )
        })
}

#[cfg(target_os = "windows")]
fn append_duplicate_operation(result: &DuplicateDeleteResult) -> Result<(), FilesystemError> {
    let path = duplicate_operation_log_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| {
            FilesystemError::new(
                "duplicate_audit_open_failed",
                format!("Unable to open duplicate operation history: {error}"),
                true,
            )
        })?;
    let line = serde_json::to_string(result).map_err(|error| {
        FilesystemError::new(
            "duplicate_audit_encode_failed",
            format!("Unable to encode duplicate operation history: {error}"),
            true,
        )
    })?;
    writeln!(file, "{line}").map_err(|error| {
        FilesystemError::new(
            "duplicate_audit_write_failed",
            format!("Unable to write duplicate operation history: {error}"),
            true,
        )
    })
}

fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn now_epoch_ms() -> u64 {
    epoch_ms(SystemTime::now()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{scan_duplicate_files, scan_storage};
    use pc_core::{DuplicateScanOptions, StorageScanOptions};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("pc-manager-p6-{label}-{unique}"))
    }

    #[test]
    fn duplicates_require_full_hash_equality() {
        let root = temp_root("full-hash");
        fs::create_dir_all(&root).expect("create root");

        let prefix = vec![b'A'; 64 * 1024];
        let suffix = vec![b'Z'; 64 * 1024];
        let mut first = prefix.clone();
        first.extend(vec![b'1'; 64 * 1024]);
        first.extend(suffix.clone());
        let mut second = prefix;
        second.extend(vec![b'2'; 64 * 1024]);
        second.extend(suffix);

        fs::write(root.join("first.bin"), first).expect("write first");
        fs::write(root.join("second.bin"), second).expect("write second");

        let cancel = AtomicBool::new(false);
        let (_, result) = scan_duplicate_files(
            std::slice::from_ref(&root),
            DuplicateScanOptions {
                min_file_bytes: 1,
                ..DuplicateScanOptions::default()
            },
            &cancel,
        )
        .expect("scan");

        assert!(result.duplicate_groups.is_empty());
        fs::remove_dir_all(root).expect("remove root");
    }

    #[test]
    fn identical_files_are_grouped_only_after_full_hash() {
        let root = temp_root("identical");
        fs::create_dir_all(&root).expect("create root");
        let bytes = vec![42_u8; 192 * 1024];
        fs::write(root.join("one.bin"), &bytes).expect("write one");
        fs::write(root.join("two.bin"), &bytes).expect("write two");

        let cancel = AtomicBool::new(false);
        let (_, result) = scan_duplicate_files(
            std::slice::from_ref(&root),
            DuplicateScanOptions {
                min_file_bytes: 1,
                ..DuplicateScanOptions::default()
            },
            &cancel,
        )
        .expect("scan");

        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.duplicate_groups[0].files.len(), 2);
        fs::remove_dir_all(root).expect("remove root");
    }

    #[test]
    fn storage_scan_aggregates_sizes_and_extensions() {
        let root = temp_root("storage");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(root.join("a.txt"), vec![1_u8; 10]).expect("write a");
        fs::write(nested.join("b.txt"), vec![2_u8; 20]).expect("write b");
        fs::write(nested.join("c.bin"), vec![3_u8; 30]).expect("write c");

        let cancel = AtomicBool::new(false);
        let result = scan_storage(
            std::slice::from_ref(&root),
            StorageScanOptions::default(),
            &cancel,
        )
        .expect("storage scan");

        assert_eq!(result.total_files, 3);
        assert_eq!(result.total_bytes, 60);
        assert_eq!(
            result
                .file_types
                .iter()
                .find(|item| item.extension == ".txt")
                .map(|item| item.bytes),
            Some(30)
        );
        fs::remove_dir_all(root).expect("remove root");
    }
}
