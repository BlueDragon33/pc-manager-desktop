use pc_core::{CleanupError, CleanupOperationRecord, CleanupPlan};
#[cfg(target_os = "windows")]
use pc_core::{CleanupExecutionError, CleanupProviderExecutionResult};
#[cfg(target_os = "windows")]
use serde_json::to_string;
#[cfg(target_os = "windows")]
use std::collections::BTreeMap;
use std::fs;
#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
#[cfg(target_os = "windows")]
use std::io::Write;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
const STALE_TEMP_AGE: Duration = Duration::from_secs(24 * 60 * 60);
#[cfg(target_os = "windows")]
const MAX_RECORDED_ERRORS: usize = 50;

#[cfg(target_os = "windows")]
#[derive(Debug, Default)]
struct ProviderAccumulator {
    requested_files: u64,
    requested_bytes: u64,
    deleted_files: u64,
    deleted_bytes: u64,
    failed_files: u64,
}

pub fn execute_cleanup_plan(plan: &CleanupPlan) -> Result<CleanupOperationRecord, CleanupError> {
    #[cfg(target_os = "windows")]
    {
        execute_windows(plan)
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

pub fn list_cleanup_operations() -> Result<Vec<CleanupOperationRecord>, CleanupError> {
    let path = operation_log_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path).map_err(|error| {
        CleanupError::new(
            "operation_log_open_failed",
            format!("Unable to open the cleanup operation log: {error}"),
            true,
        )
    })?;

    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|error| {
            CleanupError::new(
                "operation_log_read_failed",
                format!("Unable to read the cleanup operation log: {error}"),
                true,
            )
        })?;

        if line.trim().is_empty() {
            continue;
        }

        if let Ok(record) = serde_json::from_str::<CleanupOperationRecord>(&line) {
            records.push(record);
        }
    }

    records.sort_by_key(|record| std::cmp::Reverse(record.completed_at_epoch_ms));
    Ok(records)
}

#[cfg(target_os = "windows")]
fn execute_windows(plan: &CleanupPlan) -> Result<CleanupOperationRecord, CleanupError> {
    ensure_operation_log_writable()?;
    let started_at_epoch_ms = now_epoch_ms();
    let operation_id = format!("cleanup-op-{}-{}", started_at_epoch_ms, std::process::id());

    let mut providers: BTreeMap<String, ProviderAccumulator> = BTreeMap::new();
    let mut errors = Vec::new();

    for item in &plan.items {
        let provider = providers.entry(item.provider_id.clone()).or_default();
        provider.requested_files = provider.requested_files.saturating_add(1);
        provider.requested_bytes = provider.requested_bytes.saturating_add(item.bytes);

        match validate_and_delete(item) {
            Ok(()) => {
                provider.deleted_files = provider.deleted_files.saturating_add(1);
                provider.deleted_bytes = provider.deleted_bytes.saturating_add(item.bytes);
            }
            Err((code, message)) => {
                provider.failed_files = provider.failed_files.saturating_add(1);
                if errors.len() < MAX_RECORDED_ERRORS {
                    errors.push(CleanupExecutionError {
                        provider_id: item.provider_id.clone(),
                        code,
                        message,
                    });
                }
            }
        }
    }

    let provider_results = providers
        .into_iter()
        .map(|(provider_id, value)| CleanupProviderExecutionResult {
            provider_id,
            requested_files: value.requested_files,
            requested_bytes: value.requested_bytes,
            deleted_files: value.deleted_files,
            deleted_bytes: value.deleted_bytes,
            failed_files: value.failed_files,
        })
        .collect::<Vec<_>>();

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

    let record = CleanupOperationRecord {
        operation_id,
        plan_id: plan.plan_id.clone(),
        started_at_epoch_ms,
        completed_at_epoch_ms: now_epoch_ms(),
        requested_files,
        requested_bytes,
        deleted_files,
        deleted_bytes,
        failed_files,
        provider_results,
        rollback_available: false,
        rollback_label: "Not restorable".to_string(),
        errors,
    };

    append_operation_record(&record)?;
    Ok(record)
}

#[cfg(target_os = "windows")]
fn validate_and_delete(item: &pc_core::CleanupPlanItem) -> Result<(), (String, String)> {
    if !is_supported_provider(&item.provider_id) {
        return Err((
            "unsupported_provider".to_string(),
            "The cleanup plan referenced an unsupported provider.".to_string(),
        ));
    }

    let root = PathBuf::from(&item.provider_root);
    let path = PathBuf::from(&item.path);

    let root_metadata = fs::symlink_metadata(&root).map_err(|_| {
        (
            "provider_root_unavailable".to_string(),
            "The provider root is no longer available.".to_string(),
        )
    })?;
    if should_skip_metadata(&root_metadata) || !root_metadata.is_dir() {
        return Err((
            "provider_root_unsafe".to_string(),
            "The provider root is no longer a safe ordinary directory.".to_string(),
        ));
    }

    let path_metadata = fs::symlink_metadata(&path).map_err(|_| {
        (
            "candidate_unavailable".to_string(),
            "A planned candidate no longer exists or cannot be inspected.".to_string(),
        )
    })?;
    if should_skip_metadata(&path_metadata) || !path_metadata.is_file() {
        return Err((
            "candidate_unsafe".to_string(),
            "A planned candidate is no longer an ordinary file.".to_string(),
        ));
    }

    let canonical_root = fs::canonicalize(&root).map_err(|_| {
        (
            "provider_root_unresolved".to_string(),
            "The provider root could not be revalidated.".to_string(),
        )
    })?;
    let canonical_path = fs::canonicalize(&path).map_err(|_| {
        (
            "candidate_unresolved".to_string(),
            "A planned candidate could not be revalidated.".to_string(),
        )
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err((
            "candidate_outside_provider_root".to_string(),
            "A planned candidate resolved outside its approved provider root.".to_string(),
        ));
    }

    if path_metadata.len() != item.bytes {
        return Err((
            "candidate_changed_since_scan".to_string(),
            "A planned candidate changed size after the preview scan and was skipped.".to_string(),
        ));
    }

    if let Some(planned_modified) = item.modified_at_epoch_ms {
        let current_modified = path_metadata.modified().ok().and_then(epoch_ms);
        if current_modified != Some(planned_modified) {
            return Err((
                "candidate_changed_since_scan".to_string(),
                "A planned candidate changed after the preview scan and was skipped.".to_string(),
            ));
        }
    }

    if item.provider_id == "windows-user-temp" {
        let modified = path_metadata.modified().map_err(|_| {
            (
                "candidate_age_unavailable".to_string(),
                "A temporary file's age could not be revalidated.".to_string(),
            )
        })?;
        if !is_stale(modified, now_epoch_ms(), STALE_TEMP_AGE) {
            return Err((
                "candidate_no_longer_stale".to_string(),
                "A temporary file is no longer eligible under the 24-hour age rule.".to_string(),
            ));
        }
    }

    fs::remove_file(&canonical_path).map_err(|error| {
        (
            "delete_failed".to_string(),
            format!("Windows could not delete a planned candidate: {error}"),
        )
    })
}

#[cfg(target_os = "windows")]
fn is_supported_provider(provider_id: &str) -> bool {
    matches!(
        provider_id,
        "windows-user-temp"
            | "chrome-cache"
            | "edge-cache"
            | "vscode-cache"
            | "discord-cache"
            | "slack-cache"
    )
}

#[cfg(target_os = "windows")]
fn should_skip_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

    metadata.file_type().is_symlink()
        || (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

#[cfg(target_os = "windows")]
fn is_stale(modified: SystemTime, now_epoch_ms: u64, required_age: Duration) -> bool {
    let Some(modified_epoch_ms) = epoch_ms(modified) else {
        return false;
    };

    now_epoch_ms.saturating_sub(modified_epoch_ms)
        >= u64::try_from(required_age.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(target_os = "windows")]
fn ensure_operation_log_writable() -> Result<(), CleanupError> {
    let path = operation_log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CleanupError::new(
                "operation_log_directory_failed",
                format!("Unable to create the local operation-log directory: {error}"),
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
            CleanupError::new(
                "operation_log_open_failed",
                format!("Unable to prepare the cleanup operation log: {error}"),
                true,
            )
        })
}

#[cfg(target_os = "windows")]
fn append_operation_record(record: &CleanupOperationRecord) -> Result<(), CleanupError> {
    let path = operation_log_path()?;

    let json = to_string(record).map_err(|error| {
        CleanupError::new(
            "operation_log_encode_failed",
            format!("Unable to encode the cleanup operation record: {error}"),
            true,
        )
    })?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| {
            CleanupError::new(
                "operation_log_open_failed",
                format!("Unable to open the cleanup operation log: {error}"),
                true,
            )
        })?;

    writeln!(file, "{json}").map_err(|error| {
        CleanupError::new(
            "operation_log_write_failed",
            format!("Unable to persist the cleanup operation record: {error}"),
            true,
        )
    })
}

fn operation_log_path() -> Result<PathBuf, CleanupError> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        CleanupError::new(
            "local_app_data_unavailable",
            "Windows LOCALAPPDATA is unavailable, so operation history cannot be stored.",
            false,
        )
    })?;

    Ok(PathBuf::from(local_app_data)
        .join("PCManager")
        .join("operations")
        .join("cleanup.jsonl"))
}

#[cfg(target_os = "windows")]
fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(target_os = "windows")]
fn now_epoch_ms() -> u64 {
    epoch_ms(SystemTime::now()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::list_cleanup_operations;

    #[test]
    fn operation_history_reader_is_non_panicking() {
        let _ = list_cleanup_operations();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_execution_deletes_only_an_unchanged_planned_file() {
        use pc_core::{CleanupPlan, CleanupPlanItem};
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pc-manager-p4b-test-{unique}"));
        fs::create_dir_all(&root).expect("create execution test root");
        let file = root.join("cache.bin");
        fs::write(&file, b"disposable-cache").expect("write execution test file");
        let metadata = fs::metadata(&file).expect("read test metadata");
        let modified_at_epoch_ms = metadata.modified().ok().and_then(super::epoch_ms);

        let plan = CleanupPlan {
            plan_id: format!("test-plan-{unique}"),
            collected_at_epoch_ms: super::now_epoch_ms(),
            items: vec![CleanupPlanItem {
                provider_id: "vscode-cache".to_string(),
                provider_root: root.to_string_lossy().to_string(),
                path: file.to_string_lossy().to_string(),
                bytes: metadata.len(),
                modified_at_epoch_ms,
                reversible: false,
            }],
        };

        let record = super::execute_cleanup_plan(&plan).expect("execute disposable test plan");

        assert_eq!(record.deleted_files, 1);
        assert_eq!(record.failed_files, 0);
        assert!(!file.exists());
        assert!(!record.rollback_available);
        fs::remove_dir_all(root).expect("remove execution test root");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_execution_skips_a_candidate_changed_after_scan() {
        use pc_core::{CleanupPlan, CleanupPlanItem};
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pc-manager-p4b-changed-{unique}"));
        fs::create_dir_all(&root).expect("create changed test root");
        let file = root.join("cache.bin");
        fs::write(&file, b"first").expect("write initial file");
        let metadata = fs::metadata(&file).expect("read initial metadata");

        let plan = CleanupPlan {
            plan_id: format!("changed-plan-{unique}"),
            collected_at_epoch_ms: super::now_epoch_ms(),
            items: vec![CleanupPlanItem {
                provider_id: "vscode-cache".to_string(),
                provider_root: root.to_string_lossy().to_string(),
                path: file.to_string_lossy().to_string(),
                bytes: metadata.len(),
                modified_at_epoch_ms: metadata.modified().ok().and_then(super::epoch_ms),
                reversible: false,
            }],
        };

        fs::write(&file, b"changed-and-longer").expect("modify candidate after scan");
        let record = super::execute_cleanup_plan(&plan).expect("execute changed test plan");

        assert_eq!(record.deleted_files, 0);
        assert_eq!(record.failed_files, 1);
        assert!(file.exists());
        fs::remove_dir_all(root).expect("remove changed test root");
    }
}
