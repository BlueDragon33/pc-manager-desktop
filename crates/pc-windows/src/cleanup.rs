#[cfg(target_os = "windows")]
use pc_core::summarize_cleanup_plan;
#[cfg(any(target_os = "windows", test))]
use pc_core::{CleanupCategory, CleanupPlanItem, CleanupProviderSummary, CleanupWarning};
use pc_core::{
    CleanupError, CleanupExecutionResult, CleanupPlan, CleanupProviderExecution,
    CleanupScanOptions, CleanupScanSummary,
};
#[cfg(any(target_os = "windows", test))]
use std::collections::BTreeSet;
#[cfg(any(target_os = "windows", test))]
use std::fs;
#[cfg(any(target_os = "windows", test))]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
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
            "Recycle Bin scanning is not enabled in P4A because a reliable native size/count provider has not been verified yet.";
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
                "Opt-in only. P4A does not scan or empty the Recycle Bin until its native provider is verified."
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

#[cfg(target_os = "windows")]
fn now_epoch_ms() -> u64 {
    epoch_ms(SystemTime::now()).unwrap_or_default()
}


#[cfg(target_os = "windows")]
pub fn execute_cleanup_plan(plan: &CleanupPlan) -> Result<CleanupExecutionResult, CleanupError> {
    use std::collections::BTreeMap;

    let started_at_epoch_ms = now_epoch_ms();
    let requested_files = u64::try_from(plan.items.len()).unwrap_or(u64::MAX);
    let requested_bytes = plan.items.iter().map(|item| item.bytes).sum();

    let mut deleted_files = 0_u64;
    let mut deleted_bytes = 0_u64;
    let mut failed_items = 0_u64;
    let mut errors = Vec::new();
    let mut provider_totals: BTreeMap<String, CleanupProviderExecution> = BTreeMap::new();

    for item in &plan.items {
        let provider = provider_totals
            .entry(item.provider_id.clone())
            .or_insert_with(|| CleanupProviderExecution {
                provider_id: item.provider_id.clone(),
                requested_files: 0,
                requested_bytes: 0,
                deleted_files: 0,
                deleted_bytes: 0,
                failed_items: 0,
            });

        provider.requested_files = provider.requested_files.saturating_add(1);
        provider.requested_bytes = provider.requested_bytes.saturating_add(item.bytes);

        match revalidate_and_delete(item, started_at_epoch_ms) {
            Ok(bytes) => {
                deleted_files = deleted_files.saturating_add(1);
                deleted_bytes = deleted_bytes.saturating_add(bytes);
                provider.deleted_files = provider.deleted_files.saturating_add(1);
                provider.deleted_bytes = provider.deleted_bytes.saturating_add(bytes);
            }
            Err(message) => {
                failed_items = failed_items.saturating_add(1);
                provider.failed_items = provider.failed_items.saturating_add(1);
                if errors.len() < 50 {
                    errors.push(format!("{}: {}", item.provider_id, message));
                }
            }
        }
    }

    let completed_at_epoch_ms = now_epoch_ms();
    Ok(CleanupExecutionResult {
        operation_id: format!(
            "cleanup-op-{}-{}",
            started_at_epoch_ms,
            std::process::id()
        ),
        plan_id: plan.plan_id.clone(),
        started_at_epoch_ms,
        completed_at_epoch_ms,
        requested_files,
        requested_bytes,
        deleted_files,
        deleted_bytes,
        failed_items,
        providers: provider_totals.into_values().collect(),
        rollback_available: false,
        rollback_summary:
            "Cache and temporary-file deletion is not automatically restorable in P4B."
                .to_string(),
        errors,
    })
}

#[cfg(not(target_os = "windows"))]
pub fn execute_cleanup_plan(_plan: &CleanupPlan) -> Result<CleanupExecutionResult, CleanupError> {
    Err(CleanupError::new(
        "unsupported_platform",
        "Smart Clean execution is currently implemented for Windows only.",
        false,
    ))
}

#[cfg(target_os = "windows")]
fn revalidate_and_delete(
    item: &CleanupPlanItem,
    now_epoch_ms: u64,
) -> Result<u64, String> {
    let path = PathBuf::from(&item.path);
    let root = PathBuf::from(&item.provider_root);

    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("candidate no longer exists or is unreadable: {error}"))?;

    if should_skip_metadata(&metadata) || !metadata.is_file() {
        return Err("candidate is no longer a regular non-reparse file".to_string());
    }

    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("provider root is unavailable: {error}"))?;
    let canonical_path = fs::canonicalize(&path)
        .map_err(|error| format!("candidate could not be canonicalized: {error}"))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err("candidate escaped its original provider root".to_string());
    }

    if metadata.len() != item.bytes {
        return Err("candidate changed size after the preview scan".to_string());
    }

    let modified_at_epoch_ms = metadata.modified().ok().and_then(epoch_ms);
    if modified_at_epoch_ms != item.modified_at_epoch_ms {
        return Err("candidate changed after the preview scan".to_string());
    }

    if item.provider_id == "windows-user-temp" {
        let modified = metadata
            .modified()
            .map_err(|error| format!("candidate modified time is unavailable: {error}"))?;
        if !is_stale(modified, now_epoch_ms, STALE_TEMP_AGE) {
            return Err("temporary file is no longer old enough for cleanup".to_string());
        }
    }

    fs::remove_file(&canonical_path)
        .map_err(|error| format!("delete failed: {error}"))?;

    Ok(metadata.len())
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
        assert!(!summary.execution_available);
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
        assert!(!summary.execution_available);
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
