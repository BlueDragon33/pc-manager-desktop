#[cfg(target_os = "windows")]
use pc_core::summarize_cleanup_plan;
#[cfg(any(target_os = "windows", test))]
use pc_core::{
    CleanupCategory, CleanupOperationIssue, CleanupOperationRecord, CleanupPlanItem,
    CleanupProviderExecutionResult, CleanupProviderSummary, CleanupRollbackCapability,
    CleanupWarning,
};
use pc_core::{CleanupError, CleanupPlan, CleanupScanOptions, CleanupScanSummary};
#[cfg(any(target_os = "windows", test))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(target_os = "windows", test))]
use std::fs::{self, OpenOptions};
#[cfg(any(target_os = "windows", test))]
use std::io::Write;
#[cfg(any(target_os = "windows", test))]
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "windows", test))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(any(target_os = "windows", test))]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(any(target_os = "windows", test))]
const STALE_TEMP_AGE: Duration = Duration::from_secs(24 * 60 * 60);
#[cfg(any(target_os = "windows", test))]
const MAX_PROVIDER_WARNINGS: usize = 20;
#[cfg(target_os = "windows")]
static PLAN_SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[cfg(any(target_os = "windows", test))]
static OPERATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone)]
struct ProviderSpec {
    id: &'static str,
    display_name: &'static str,
    category: CleanupCategory,
    roots: Vec<PathBuf>,
    enabled_by_default: bool,
    reversible: bool,
    description: &'static str,
    stale_age: Option<Duration>,
}

pub fn scan_cleanup_candidates(
    options: CleanupScanOptions,
) -> Result<(CleanupPlan, CleanupScanSummary), CleanupError> {
    #[cfg(target_os = "windows")]
    {
        scan_windows(options)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = options;
        Err(CleanupError::new(
            "unsupported_platform",
            "Smart Clean scanning is currently implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn scan_windows(
    options: CleanupScanOptions,
) -> Result<(CleanupPlan, CleanupScanSummary), CleanupError> {
    let collected_at_epoch_ms = now_epoch_ms();
    let sequence = PLAN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let plan_id = format!(
        "cleanup-{}-{}-{}",
        collected_at_epoch_ms,
        std::process::id(),
        sequence
    );

    let providers = provider_specs();
    let mut all_items = Vec::new();
    let mut summaries = Vec::new();
    let mut all_warnings = Vec::new();

    for provider in providers {
        let (summary, items, warnings) = scan_provider(&provider, collected_at_epoch_ms);
        all_items.extend(items);
        all_warnings.extend(warnings);
        summaries.push(summary);
    }

    let recycle_summary = if options.include_recycle_bin {
        let warning =
            "Recycle Bin scanning is not enabled in P4B because a reliable native size/count provider has not been verified yet.";
        all_warnings.push(CleanupWarning {
            provider_id: "recycle-bin".to_string(),
            message: warning.to_string(),
        });
        CleanupProviderSummary {
            provider_id: "recycle-bin".to_string(),
            display_name: "Recycle Bin".to_string(),
            category: CleanupCategory::RecycleBin,
            enabled_by_default: false,
            available: false,
            file_count: 0,
            bytes: 0,
            reversible: false,
            description:
                "Opt-in only. P4B does not scan or empty the Recycle Bin until its native provider is verified."
                    .to_string(),
            warnings: vec![warning.to_string()],
        }
    } else {
        CleanupProviderSummary {
            provider_id: "recycle-bin".to_string(),
            display_name: "Recycle Bin".to_string(),
            category: CleanupCategory::RecycleBin,
            enabled_by_default: false,
            available: false,
            file_count: 0,
            bytes: 0,
            reversible: false,
            description: "Opt-in provider. Disabled for this preview scan.".to_string(),
            warnings: vec![],
        }
    };
    summaries.push(recycle_summary);

    let plan = CleanupPlan {
        plan_id: plan_id.clone(),
        collected_at_epoch_ms,
        items: all_items,
    };
    let summary = summarize_cleanup_plan(plan_id, collected_at_epoch_ms, summaries, all_warnings);

    Ok((plan, summary))
}

#[cfg(target_os = "windows")]
fn provider_specs() -> Vec<ProviderSpec> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming_app_data = std::env::var_os("APPDATA").map(PathBuf::from);

    let mut temp_roots = vec![std::env::temp_dir()];
    if let Some(local) = local_app_data.as_ref() {
        temp_roots.push(local.join("Temp"));
    }

    let chrome_roots = local_app_data
        .as_ref()
        .map(|local| chromium_cache_roots(&local.join("Google").join("Chrome").join("User Data")))
        .unwrap_or_default();

    let edge_roots = local_app_data
        .as_ref()
        .map(|local| chromium_cache_roots(&local.join("Microsoft").join("Edge").join("User Data")))
        .unwrap_or_default();

    let code_roots = roaming_app_data
        .as_ref()
        .map(|roaming| {
            vec![
                roaming.join("Code").join("Cache"),
                roaming.join("Code").join("CachedData"),
            ]
        })
        .unwrap_or_default();

    let discord_roots = roaming_app_data
        .as_ref()
        .map(|roaming| vec![roaming.join("discord").join("Cache")])
        .unwrap_or_default();

    let slack_roots = roaming_app_data
        .as_ref()
        .map(|roaming| vec![roaming.join("Slack").join("Cache")])
        .unwrap_or_default();

    vec![
        ProviderSpec {
            id: "windows-user-temp",
            display_name: "Windows user temp",
            category: CleanupCategory::WindowsTemp,
            roots: deduplicate_roots(temp_roots),
            enabled_by_default: true,
            reversible: false,
            description: "User-level temporary files older than 24 hours.",
            stale_age: Some(STALE_TEMP_AGE),
        },
        ProviderSpec {
            id: "chrome-cache",
            display_name: "Google Chrome cache",
            category: CleanupCategory::BrowserCache,
            roots: deduplicate_roots(chrome_roots),
            enabled_by_default: true,
            reversible: false,
            description:
                "Cache directories only. History, cookies, passwords, bookmarks, and sessions are excluded.",
            stale_age: None,
        },
        ProviderSpec {
            id: "edge-cache",
            display_name: "Microsoft Edge cache",
            category: CleanupCategory::BrowserCache,
            roots: deduplicate_roots(edge_roots),
            enabled_by_default: true,
            reversible: false,
            description:
                "Cache directories only. History, cookies, passwords, bookmarks, and sessions are excluded.",
            stale_age: None,
        },
        ProviderSpec {
            id: "vscode-cache",
            display_name: "Visual Studio Code cache",
            category: CleanupCategory::ApplicationCache,
            roots: deduplicate_roots(code_roots),
            enabled_by_default: true,
            reversible: false,
            description: "Explicit Visual Studio Code cache directories only.",
            stale_age: None,
        },
        ProviderSpec {
            id: "discord-cache",
            display_name: "Discord cache",
            category: CleanupCategory::ApplicationCache,
            roots: deduplicate_roots(discord_roots),
            enabled_by_default: true,
            reversible: false,
            description: "Explicit Discord cache directory only.",
            stale_age: None,
        },
        ProviderSpec {
            id: "slack-cache",
            display_name: "Slack cache",
            category: CleanupCategory::ApplicationCache,
            roots: deduplicate_roots(slack_roots),
            enabled_by_default: true,
            reversible: false,
            description: "Explicit Slack cache directory only.",
            stale_age: None,
        },
    ]
}

#[cfg(target_os = "windows")]
fn chromium_cache_roots(user_data_root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(user_data_root) else {
        return Vec::new();
    };

    let mut roots = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != "Default" && !name.starts_with("Profile ") {
            continue;
        }

        let profile = entry.path();
        roots.push(profile.join("Cache"));
        roots.push(profile.join("Code Cache"));
        roots.push(profile.join("GPUCache"));
    }

    roots
}

#[cfg(any(target_os = "windows", test))]
fn deduplicate_roots(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut deduplicated = Vec::new();

    for root in roots {
        let key = root
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        if seen.insert(key) {
            deduplicated.push(root);
        }
    }

    deduplicated
}

#[cfg(any(target_os = "windows", test))]
fn scan_provider(
    provider: &ProviderSpec,
    now_epoch_ms: u64,
) -> (
    CleanupProviderSummary,
    Vec<CleanupPlanItem>,
    Vec<CleanupWarning>,
) {
    let mut items = Vec::new();
    let mut provider_warning_messages = Vec::new();
    let mut global_warnings = Vec::new();
    let mut available = false;

    for root in &provider.roots {
        if !root.exists() {
            continue;
        }
        available = true;

        let root_string = root.to_string_lossy().to_string();
        scan_root(
            provider,
            root,
            &root_string,
            now_epoch_ms,
            &mut items,
            &mut provider_warning_messages,
        );
    }

    for message in &provider_warning_messages {
        global_warnings.push(CleanupWarning {
            provider_id: provider.id.to_string(),
            message: message.clone(),
        });
    }

    let file_count = u64::try_from(items.len()).unwrap_or(u64::MAX);
    let bytes = items.iter().map(|item| item.bytes).sum();

    (
        CleanupProviderSummary {
            provider_id: provider.id.to_string(),
            display_name: provider.display_name.to_string(),
            category: provider.category,
            enabled_by_default: provider.enabled_by_default,
            available,
            file_count,
            bytes,
            reversible: provider.reversible,
            description: provider.description.to_string(),
            warnings: provider_warning_messages,
        },
        items,
        global_warnings,
    )
}

#[cfg(any(target_os = "windows", test))]
fn scan_root(
    provider: &ProviderSpec,
    root: &Path,
    provider_root: &str,
    now_epoch_ms: u64,
    items: &mut Vec<CleanupPlanItem>,
    warnings: &mut Vec<String>,
) {
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                record_warning(
                    warnings,
                    format!("Unable to inspect {}: {error}", path.display()),
                );
                continue;
            }
        };

        if should_skip_metadata(&metadata) {
            continue;
        }

        if metadata.is_dir() {
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(error) => {
                    record_warning(
                        warnings,
                        format!("Unable to read {}: {error}", path.display()),
                    );
                    continue;
                }
            };

            for entry in entries {
                match entry {
                    Ok(entry) => stack.push(entry.path()),
                    Err(error) => record_warning(
                        warnings,
                        format!(
                            "Unable to enumerate an item under {}: {error}",
                            path.display()
                        ),
                    ),
                }
            }
            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        if let Some(required_age) = provider.stale_age {
            let Some(modified) = metadata.modified().ok() else {
                continue;
            };
            if !is_stale(modified, now_epoch_ms, required_age) {
                continue;
            }
        }

        let modified_at_epoch_ms = metadata.modified().ok().and_then(epoch_ms);

        items.push(CleanupPlanItem {
            provider_id: provider.id.to_string(),
            provider_root: provider_root.to_string(),
            path: path.to_string_lossy().to_string(),
            bytes: metadata.len(),
            modified_at_epoch_ms,
            reversible: provider.reversible,
        });
    }
}

#[cfg(any(target_os = "windows", test))]
fn record_warning(warnings: &mut Vec<String>, message: String) {
    if warnings.len() < MAX_PROVIDER_WARNINGS {
        warnings.push(message);
    } else if warnings.len() == MAX_PROVIDER_WARNINGS {
        warnings.push("Additional inaccessible items were omitted from this preview.".to_string());
    }
}

#[cfg(any(target_os = "windows", test))]
fn is_stale(modified: SystemTime, now_epoch_ms: u64, required_age: Duration) -> bool {
    let Some(modified_epoch_ms) = epoch_ms(modified) else {
        return false;
    };

    now_epoch_ms.saturating_sub(modified_epoch_ms)
        >= u64::try_from(required_age.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(any(target_os = "windows", test))]
fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(target_os = "windows")]
fn should_skip_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

    metadata.file_type().is_symlink()
        || (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

#[cfg(all(test, not(target_os = "windows")))]
fn should_skip_metadata(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(any(target_os = "windows", test))]
fn now_epoch_ms() -> u64 {
    epoch_ms(SystemTime::now()).unwrap_or_default()
}


pub fn execute_cleanup_plan(plan: &CleanupPlan) -> Result<CleanupOperationRecord, CleanupError> {
    #[cfg(target_os = "windows")]
    {
        let audit_dir = operation_log_dir()?;
        execute_plan_with_specs(plan, &provider_specs(), &audit_dir)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = plan;
        Err(CleanupError::new(
            "unsupported_platform",
            "Cleanup execution is currently implemented for Windows only.",
            false,
        ))
    }
}

pub fn load_cleanup_operations(
    limit: usize,
) -> Result<Vec<CleanupOperationRecord>, CleanupError> {
    #[cfg(target_os = "windows")]
    {
        let audit_dir = operation_log_dir()?;
        load_operations_from_dir(&audit_dir, limit)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = limit;
        Err(CleanupError::new(
            "unsupported_platform",
            "Cleanup operation history is currently implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn operation_log_dir() -> Result<PathBuf, CleanupError> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        CleanupError::new(
            "audit_location_unavailable",
            "Windows LOCALAPPDATA is unavailable, so cleanup execution is blocked.",
            false,
        )
    })?;
    let path = PathBuf::from(local_app_data)
        .join("PCManagerDesktop")
        .join("operations");
    fs::create_dir_all(&path).map_err(|error| {
        CleanupError::new(
            "audit_directory_unavailable",
            format!("Unable to prepare the local cleanup audit directory: {error}"),
            false,
        )
    })?;
    Ok(path)
}

#[cfg(any(target_os = "windows", test))]
fn execute_plan_with_specs(
    plan: &CleanupPlan,
    providers: &[ProviderSpec],
    audit_dir: &Path,
) -> Result<CleanupOperationRecord, CleanupError> {
    fs::create_dir_all(audit_dir).map_err(|error| {
        CleanupError::new(
            "audit_directory_unavailable",
            format!("Unable to prepare the cleanup audit directory: {error}"),
            false,
        )
    })?;

    let started_at_epoch_ms = now_epoch_ms();
    let sequence = OPERATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let operation_id = format!(
        "cleanup-operation-{}-{}-{}",
        started_at_epoch_ms,
        std::process::id(),
        sequence
    );

    let pending_path = audit_dir.join(format!("{operation_id}.pending"));
    write_pending_record(
        &pending_path,
        &operation_id,
        &plan.plan_id,
        started_at_epoch_ms,
    )?;

    let mut provider_results = BTreeMap::<String, CleanupProviderExecutionResult>::new();
    for item in &plan.items {
        let result = provider_results
            .entry(item.provider_id.clone())
            .or_insert_with(|| CleanupProviderExecutionResult {
                provider_id: item.provider_id.clone(),
                requested_files: 0,
                requested_bytes: 0,
                deleted_files: 0,
                deleted_bytes: 0,
                failed_files: 0,
            });
        result.requested_files = result.requested_files.saturating_add(1);
        result.requested_bytes = result.requested_bytes.saturating_add(item.bytes);
    }

    let mut issues = BTreeMap::<(String, String), u64>::new();
    let now_ms = now_epoch_ms();

    for item in &plan.items {
        let provider_result = provider_results
            .get_mut(&item.provider_id)
            .expect("provider aggregate exists for each planned item");

        match revalidate_plan_item(item, providers, now_ms) {
            Ok((canonical_path, current_bytes)) => match fs::remove_file(&canonical_path) {
                Ok(()) => {
                    provider_result.deleted_files =
                        provider_result.deleted_files.saturating_add(1);
                    provider_result.deleted_bytes =
                        provider_result.deleted_bytes.saturating_add(current_bytes);
                }
                Err(_) => {
                    provider_result.failed_files =
                        provider_result.failed_files.saturating_add(1);
                    record_issue(
                        &mut issues,
                        "delete_failed",
                        "A verified cleanup candidate could not be deleted.",
                    );
                }
            },
            Err((code, message)) => {
                provider_result.failed_files = provider_result.failed_files.saturating_add(1);
                record_issue(&mut issues, code, message);
            }
        }
    }

    let provider_results = provider_results.into_values().collect::<Vec<_>>();
    let requested_files = provider_results
        .iter()
        .map(|result| result.requested_files)
        .sum();
    let requested_bytes = provider_results
        .iter()
        .map(|result| result.requested_bytes)
        .sum();
    let deleted_files = provider_results
        .iter()
        .map(|result| result.deleted_files)
        .sum();
    let deleted_bytes = provider_results
        .iter()
        .map(|result| result.deleted_bytes)
        .sum();
    let failed_files = provider_results
        .iter()
        .map(|result| result.failed_files)
        .sum();

    let operation = CleanupOperationRecord {
        operation_id: operation_id.clone(),
        plan_id: plan.plan_id.clone(),
        started_at_epoch_ms,
        completed_at_epoch_ms: now_epoch_ms(),
        requested_files,
        requested_bytes,
        deleted_files,
        deleted_bytes,
        failed_files,
        provider_results,
        rollback_capability: CleanupRollbackCapability::NotRestorable,
        issues: issues
            .into_iter()
            .map(|((code, message), count)| CleanupOperationIssue {
                code,
                message,
                count,
            })
            .collect(),
    };

    persist_completed_operation(audit_dir, &pending_path, &operation)?;
    Ok(operation)
}

#[cfg(any(target_os = "windows", test))]
fn revalidate_plan_item(
    item: &CleanupPlanItem,
    providers: &[ProviderSpec],
    now_epoch_ms: u64,
) -> Result<(PathBuf, u64), (&'static str, &'static str)> {
    let provider = providers
        .iter()
        .find(|provider| provider.id == item.provider_id)
        .ok_or((
            "provider_not_allowed",
            "The cleanup provider is no longer on the built-in allow-list.",
        ))?;

    let planned_root = PathBuf::from(&item.provider_root);
    let planned_path = PathBuf::from(&item.path);

    let root_metadata = fs::symlink_metadata(&planned_root).map_err(|_| {
        (
            "provider_root_unavailable",
            "The original cleanup provider root is no longer available.",
        )
    })?;
    if should_skip_metadata(&root_metadata) || !root_metadata.is_dir() {
        return Err((
            "provider_root_not_safe",
            "The cleanup provider root is no longer a safe directory.",
        ));
    }

    let canonical_root = fs::canonicalize(&planned_root).map_err(|_| {
        (
            "provider_root_unavailable",
            "The cleanup provider root could not be resolved safely.",
        )
    })?;

    let root_still_allowed = provider.roots.iter().any(|current_root| {
        fs::canonicalize(current_root)
            .map(|current| current == canonical_root)
            .unwrap_or(false)
    });
    if !root_still_allowed {
        return Err((
            "provider_root_changed",
            "The cleanup provider root no longer matches the built-in allow-list.",
        ));
    }

    let metadata = fs::symlink_metadata(&planned_path).map_err(|_| {
        (
            "candidate_missing",
            "A cleanup candidate no longer exists or is inaccessible.",
        )
    })?;
    if should_skip_metadata(&metadata) {
        return Err((
            "candidate_reparse_point",
            "A cleanup candidate became a symlink or reparse point after scanning.",
        ));
    }
    if !metadata.is_file() {
        return Err((
            "candidate_not_file",
            "A cleanup candidate is no longer a regular file.",
        ));
    }

    let canonical_path = fs::canonicalize(&planned_path).map_err(|_| {
        (
            "candidate_unresolvable",
            "A cleanup candidate could not be resolved safely.",
        )
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err((
            "candidate_outside_root",
            "A cleanup candidate resolved outside its original provider root.",
        ));
    }

    if metadata.len() != item.bytes {
        return Err((
            "candidate_changed",
            "A cleanup candidate changed size after the preview scan.",
        ));
    }

    if let Some(planned_modified) = item.modified_at_epoch_ms {
        let current_modified = metadata.modified().ok().and_then(epoch_ms);
        if current_modified != Some(planned_modified) {
            return Err((
                "candidate_changed",
                "A cleanup candidate changed after the preview scan.",
            ));
        }
    }

    if let Some(required_age) = provider.stale_age {
        let modified = metadata.modified().map_err(|_| {
            (
                "candidate_age_unavailable",
                "The temporary file age could not be revalidated.",
            )
        })?;
        if !is_stale(modified, now_epoch_ms, required_age) {
            return Err((
                "candidate_no_longer_stale",
                "A temporary file no longer meets the stale-file policy.",
            ));
        }
    }

    Ok((canonical_path, metadata.len()))
}

#[cfg(any(target_os = "windows", test))]
fn record_issue(
    issues: &mut BTreeMap<(String, String), u64>,
    code: &str,
    message: &str,
) {
    let count = issues
        .entry((code.to_string(), message.to_string()))
        .or_insert(0);
    *count = count.saturating_add(1);
}

#[cfg(any(target_os = "windows", test))]
fn write_pending_record(
    path: &Path,
    operation_id: &str,
    plan_id: &str,
    started_at_epoch_ms: u64,
) -> Result<(), CleanupError> {
    let payload = serde_json::json!({
        "operationId": operation_id,
        "planId": plan_id,
        "startedAtEpochMs": started_at_epoch_ms,
        "state": "inProgress"
    });
    let bytes = serde_json::to_vec(&payload).map_err(|error| {
        CleanupError::new(
            "audit_serialize_failed",
            format!("Unable to serialize the cleanup audit preflight: {error}"),
            false,
        )
    })?;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            CleanupError::new(
                "audit_preflight_failed",
                format!("Unable to create the cleanup audit preflight: {error}"),
                false,
            )
        })?;
    file.write_all(&bytes).map_err(|error| {
        CleanupError::new(
            "audit_preflight_failed",
            format!("Unable to write the cleanup audit preflight: {error}"),
            false,
        )
    })?;
    file.sync_all().map_err(|error| {
        CleanupError::new(
            "audit_preflight_failed",
            format!("Unable to flush the cleanup audit preflight: {error}"),
            false,
        )
    })
}

#[cfg(any(target_os = "windows", test))]
fn persist_completed_operation(
    audit_dir: &Path,
    pending_path: &Path,
    operation: &CleanupOperationRecord,
) -> Result<(), CleanupError> {
    let temp_path = audit_dir.join(format!("{}.complete.tmp", operation.operation_id));
    let final_path = audit_dir.join(format!("{}.json", operation.operation_id));
    let bytes = serde_json::to_vec(operation).map_err(|error| {
        CleanupError::new(
            "audit_serialize_failed",
            format!("Unable to serialize the cleanup operation record: {error}"),
            false,
        )
    })?;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .map_err(|error| {
            CleanupError::new(
                "audit_write_failed",
                format!("Unable to prepare the completed cleanup record: {error}"),
                false,
            )
        })?;
    file.write_all(&bytes).map_err(|error| {
        CleanupError::new(
            "audit_write_failed",
            format!("Unable to write the completed cleanup record: {error}"),
            false,
        )
    })?;
    file.sync_all().map_err(|error| {
        CleanupError::new(
            "audit_write_failed",
            format!("Unable to flush the completed cleanup record: {error}"),
            false,
        )
    })?;
    fs::rename(&temp_path, &final_path).map_err(|error| {
        CleanupError::new(
            "audit_commit_failed",
            format!("Unable to commit the completed cleanup record: {error}"),
            false,
        )
    })?;

    let _ = fs::remove_file(pending_path);
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn load_operations_from_dir(
    audit_dir: &Path,
    limit: usize,
) -> Result<Vec<CleanupOperationRecord>, CleanupError> {
    if !audit_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(audit_dir).map_err(|error| {
        CleanupError::new(
            "audit_read_failed",
            format!("Unable to read cleanup operation history: {error}"),
            true,
        )
    })?;
    let mut operations = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if let Ok(operation) = serde_json::from_slice::<CleanupOperationRecord>(&bytes) {
            operations.push(operation);
        }
    }

    operations.sort_by(|a, b| b.completed_at_epoch_ms.cmp(&a.completed_at_epoch_ms));
    operations.truncate(limit.min(100));
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::{
        deduplicate_roots, is_stale, scan_provider, CleanupCategory, ProviderSpec, STALE_TEMP_AGE,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn duplicate_roots_are_collapsed() {
        let roots = deduplicate_roots(vec![
            PathBuf::from("C:/Users/Test/AppData/Local/Temp"),
            PathBuf::from("C:/Users/Test/AppData/Local/Temp"),
        ]);

        assert_eq!(roots.len(), 1);
    }

    #[test]
    fn stale_threshold_is_conservative() {
        let now_ms = 2 * 24 * 60 * 60 * 1000;
        let old = UNIX_EPOCH + Duration::from_secs(60);
        let recent = UNIX_EPOCH + Duration::from_millis(now_ms - 60_000);

        assert!(is_stale(old, now_ms, STALE_TEMP_AGE));
        assert!(!is_stale(recent, now_ms, STALE_TEMP_AGE));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cleanup_preview_scans_real_builtin_roots_without_execution() {
        let (plan, summary) =
            super::scan_cleanup_candidates(pc_core::CleanupScanOptions::default())
                .expect("Windows cleanup preview should run");

        assert!(!summary.plan_id.is_empty());
        assert_eq!(summary.plan_id, plan.plan_id);
        assert_eq!(summary.execution_available, summary.total_files > 0);
        assert!(summary
            .providers
            .iter()
            .any(|provider| provider.provider_id == "windows-user-temp"));

        let planned_bytes: u64 = plan.items.iter().map(|item| item.bytes).sum();
        let planned_files = u64::try_from(plan.items.len()).expect("plan item count fits u64");
        assert_eq!(summary.total_bytes, planned_bytes);
        assert_eq!(summary.total_files, planned_files);
    }

    #[test]
    fn cleanup_execution_deletes_only_unchanged_files_inside_allowed_root() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pc-manager-execute-test-{unique}"));
        let audit = root.join("audit");
        let provider_root = root.join("cache");
        fs::create_dir_all(&provider_root).expect("create provider root");
        let candidate = provider_root.join("candidate.tmp");
        fs::write(&candidate, b"disposable-cache").expect("write disposable candidate");
        let metadata = fs::metadata(&candidate).expect("candidate metadata");
        let modified_at_epoch_ms = metadata.modified().ok().and_then(super::epoch_ms);

        let provider = ProviderSpec {
            id: "test-provider",
            display_name: "Test provider",
            category: CleanupCategory::ApplicationCache,
            roots: vec![provider_root.clone()],
            enabled_by_default: true,
            reversible: false,
            description: "test",
            stale_age: None,
        };
        let plan = pc_core::CleanupPlan {
            plan_id: "test-execute-plan".to_string(),
            collected_at_epoch_ms: super::now_epoch_ms(),
            items: vec![pc_core::CleanupPlanItem {
                provider_id: "test-provider".to_string(),
                provider_root: provider_root.to_string_lossy().to_string(),
                path: candidate.to_string_lossy().to_string(),
                bytes: metadata.len(),
                modified_at_epoch_ms,
                reversible: false,
            }],
        };

        let operation =
            super::execute_plan_with_specs(&plan, &[provider], &audit).expect("execute test plan");

        assert!(!candidate.exists());
        assert_eq!(operation.requested_files, 1);
        assert_eq!(operation.deleted_files, 1);
        assert_eq!(operation.failed_files, 0);
        assert_eq!(operation.rollback_capability, pc_core::CleanupRollbackCapability::NotRestorable);

        let history = super::load_operations_from_dir(&audit, 20).expect("load audit history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].operation_id, operation.operation_id);

        fs::remove_dir_all(root).expect("remove execution test root");
    }

    #[test]
    fn cleanup_execution_refuses_candidate_outside_provider_root() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pc-manager-boundary-test-{unique}"));
        let audit = root.join("audit");
        let provider_root = root.join("cache");
        let outside_root = root.join("documents");
        fs::create_dir_all(&provider_root).expect("create provider root");
        fs::create_dir_all(&outside_root).expect("create outside root");
        let outside_file = outside_root.join("keep.txt");
        fs::write(&outside_file, b"must-remain").expect("write outside file");
        let metadata = fs::metadata(&outside_file).expect("outside metadata");

        let provider = ProviderSpec {
            id: "test-provider",
            display_name: "Test provider",
            category: CleanupCategory::ApplicationCache,
            roots: vec![provider_root.clone()],
            enabled_by_default: true,
            reversible: false,
            description: "test",
            stale_age: None,
        };
        let plan = pc_core::CleanupPlan {
            plan_id: "test-boundary-plan".to_string(),
            collected_at_epoch_ms: super::now_epoch_ms(),
            items: vec![pc_core::CleanupPlanItem {
                provider_id: "test-provider".to_string(),
                provider_root: provider_root.to_string_lossy().to_string(),
                path: outside_file.to_string_lossy().to_string(),
                bytes: metadata.len(),
                modified_at_epoch_ms: metadata.modified().ok().and_then(super::epoch_ms),
                reversible: false,
            }],
        };

        let operation =
            super::execute_plan_with_specs(&plan, &[provider], &audit).expect("execute boundary plan");

        assert!(outside_file.exists());
        assert_eq!(operation.deleted_files, 0);
        assert_eq!(operation.failed_files, 1);
        assert!(operation
            .issues
            .iter()
            .any(|issue| issue.code == "candidate_outside_root"));

        fs::remove_dir_all(root).expect("remove boundary test root");
    }

    #[test]
    fn provider_scan_aggregates_file_bytes_exactly() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pc-manager-cleanup-test-{unique}"));
        fs::create_dir_all(&root).expect("create test root");
        fs::write(root.join("one.tmp"), b"1234").expect("write first file");
        fs::write(root.join("two.tmp"), b"123456").expect("write second file");

        let provider = ProviderSpec {
            id: "test-provider",
            display_name: "Test provider",
            category: CleanupCategory::ApplicationCache,
            roots: vec![root.clone()],
            enabled_by_default: true,
            reversible: false,
            description: "test",
            stale_age: None,
        };

        let (summary, items, warnings) = scan_provider(&provider, u64::MAX);

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.bytes, 10);
        assert_eq!(items.len(), 2);
        assert!(warnings.is_empty());

        fs::remove_dir_all(root).expect("remove test root");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cleanup_preview_returns_a_read_only_native_plan() {
        let (plan, summary) =
            super::scan_cleanup_candidates(Default::default()).expect("cleanup preview should run");

        assert_eq!(plan.plan_id, summary.plan_id);
        assert_eq!(summary.execution_available, summary.total_files > 0);
        assert_eq!(
            u64::try_from(plan.items.len()).expect("candidate count fits in u64"),
            summary.total_files
        );
        assert_eq!(
            plan.items.iter().map(|item| item.bytes).sum::<u64>(),
            summary.total_bytes
        );
        assert!(plan
            .items
            .iter()
            .all(|item| !item.provider_root.trim().is_empty()));
    }
}
