use pc_core::{
    evaluate_health, AppInfo, AppsError, CleanupError, CleanupOperationRecord, CleanupPlanStore,
    CleanupScanOptions, CleanupScanSummary, DuplicateDeleteRequest, DuplicateDeleteResult,
    DuplicateScanOptions, DuplicateScanPlanStore, DuplicateScanSummary, FilesystemError,
    HealthReport, InstalledAppEntry, InventoryError, ScanRootSelection, StartupChangeRequest,
    StartupEntry, StartupError, StartupOperationRecord, StorageScanOptions, StorageScanSummary,
    SystemInventory, UninstallLaunchResult, UninstallRequest,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::State;

#[derive(Debug, Default)]
struct ScanRootStore {
    roots: HashMap<String, PathBuf>,
}

#[derive(Debug, Default)]
struct ScanCancellationStore {
    scans: HashMap<String, Arc<AtomicBool>>,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo::current()
}

#[tauri::command]
fn get_system_inventory() -> Result<SystemInventory, InventoryError> {
    pc_windows::collect_system_inventory()
}

#[tauri::command]
fn run_health_check() -> Result<HealthReport, InventoryError> {
    let inventory = pc_windows::collect_system_inventory()?;
    Ok(evaluate_health(&inventory))
}

#[tauri::command]
fn scan_cleanup_candidates(
    options: CleanupScanOptions,
    cleanup_plans: State<'_, Mutex<CleanupPlanStore>>,
) -> Result<CleanupScanSummary, CleanupError> {
    let (plan, summary) = pc_windows::scan_cleanup_candidates(options)?;
    let mut stored_plans = cleanup_plans.lock().map_err(|_| {
        CleanupError::new(
            "cleanup_plan_store_unavailable",
            "The native cleanup-plan store is unavailable.",
            true,
        )
    })?;

    stored_plans.replace(plan);
    Ok(summary)
}

#[tauri::command]
fn execute_cleanup_plan(
    plan_id: String,
    cleanup_plans: State<'_, Mutex<CleanupPlanStore>>,
) -> Result<CleanupOperationRecord, CleanupError> {
    let plan = {
        let mut stored_plans = cleanup_plans.lock().map_err(|_| {
            CleanupError::new(
                "cleanup_plan_store_unavailable",
                "The native cleanup-plan store is unavailable.",
                true,
            )
        })?;

        stored_plans.take_by_id(&plan_id).ok_or_else(|| {
            CleanupError::new(
                "cleanup_plan_not_found",
                "The cleanup plan is missing, expired, or has already been executed. Run a new preview scan.",
                true,
            )
        })?
    };

    pc_windows::execute_cleanup_plan(&plan)
}

#[tauri::command]
fn list_cleanup_operations() -> Result<Vec<CleanupOperationRecord>, CleanupError> {
    pc_windows::list_cleanup_operations()
}

#[tauri::command]
fn list_startup_entries() -> Result<Vec<StartupEntry>, StartupError> {
    pc_windows::list_startup_entries()
}

#[tauri::command]
fn set_startup_entry_enabled(
    request: StartupChangeRequest,
) -> Result<StartupOperationRecord, StartupError> {
    pc_windows::set_startup_entry_enabled(&request)
}

#[tauri::command]
fn list_startup_operations() -> Result<Vec<StartupOperationRecord>, StartupError> {
    pc_windows::list_startup_operations()
}

#[tauri::command]
fn list_installed_apps() -> Result<Vec<InstalledAppEntry>, AppsError> {
    pc_windows::list_installed_app_entries()
}

#[tauri::command]
fn launch_uninstall(request: UninstallRequest) -> Result<UninstallLaunchResult, AppsError> {
    pc_windows::launch_standard_uninstall(&request)
}

#[tauri::command]
fn select_scan_root(
    scan_roots: State<'_, Mutex<ScanRootStore>>,
) -> Result<Option<ScanRootSelection>, FilesystemError> {
    let Some(path) = pc_windows::select_user_scan_root()? else {
        return Ok(None);
    };
    let id = pc_windows::scan_root_id(&path);
    let display_path = path.to_string_lossy().to_string();

    let mut roots = scan_roots.lock().map_err(|_| {
        FilesystemError::new(
            "scan_root_store_unavailable",
            "The native scan-root store is unavailable.",
            true,
        )
    })?;
    roots.roots.insert(id.clone(), path);

    Ok(Some(ScanRootSelection { id, display_path }))
}

#[tauri::command]
async fn scan_duplicates(
    request_id: String,
    root_ids: Vec<String>,
    options: DuplicateScanOptions,
    scan_roots: State<'_, Mutex<ScanRootStore>>,
    duplicate_plans: State<'_, Mutex<DuplicateScanPlanStore>>,
    cancellations: State<'_, Mutex<ScanCancellationStore>>,
) -> Result<DuplicateScanSummary, FilesystemError> {
    validate_request_id(&request_id)?;
    let roots = resolve_scan_roots(&root_ids, &scan_roots)?;
    let cancel = register_scan(&request_id, &cancellations)?;
    let cancel_for_scan = Arc::clone(&cancel);

    let task = tauri::async_runtime::spawn_blocking(move || {
        pc_windows::scan_duplicate_files(&roots, options, cancel_for_scan.as_ref())
    });
    let joined = task.await;

    remove_scan(&request_id, &cancellations);
    let (plan, summary) = joined
        .map_err(|error| {
            FilesystemError::new(
                "duplicate_scan_task_failed",
                format!("The duplicate scan task failed: {error}"),
                true,
            )
        })??;

    let mut stored_plans = duplicate_plans.lock().map_err(|_| {
        FilesystemError::new(
            "duplicate_plan_store_unavailable",
            "The native duplicate-scan plan store is unavailable.",
            true,
        )
    })?;
    stored_plans.replace(plan);
    Ok(summary)
}

#[tauri::command]
fn delete_duplicate_files(
    request: DuplicateDeleteRequest,
    duplicate_plans: State<'_, Mutex<DuplicateScanPlanStore>>,
) -> Result<DuplicateDeleteResult, FilesystemError> {
    let plan = {
        let stored_plans = duplicate_plans.lock().map_err(|_| {
            FilesystemError::new(
                "duplicate_plan_store_unavailable",
                "The native duplicate-scan plan store is unavailable.",
                true,
            )
        })?;
        stored_plans
            .get_by_id(&request.scan_id)
            .cloned()
            .ok_or_else(|| {
                FilesystemError::new(
                    "duplicate_plan_not_found",
                    "The duplicate scan is missing or stale. Run a new scan before deleting files.",
                    true,
                )
            })?
    };

    pc_windows::delete_duplicate_files(&plan, &request.file_ids)
}

#[tauri::command]
async fn scan_storage(
    request_id: String,
    root_ids: Vec<String>,
    options: StorageScanOptions,
    scan_roots: State<'_, Mutex<ScanRootStore>>,
    cancellations: State<'_, Mutex<ScanCancellationStore>>,
) -> Result<StorageScanSummary, FilesystemError> {
    validate_request_id(&request_id)?;
    let roots = resolve_scan_roots(&root_ids, &scan_roots)?;
    let cancel = register_scan(&request_id, &cancellations)?;
    let cancel_for_scan = Arc::clone(&cancel);

    let task = tauri::async_runtime::spawn_blocking(move || {
        pc_windows::scan_storage(&roots, options, cancel_for_scan.as_ref())
    });
    let joined = task.await;

    remove_scan(&request_id, &cancellations);
    joined.map_err(|error| {
        FilesystemError::new(
            "storage_scan_task_failed",
            format!("The storage scan task failed: {error}"),
            true,
        )
    })?
}

#[tauri::command]
fn cancel_filesystem_scan(
    request_id: String,
    cancellations: State<'_, Mutex<ScanCancellationStore>>,
) -> Result<bool, FilesystemError> {
    validate_request_id(&request_id)?;
    let scans = cancellations.lock().map_err(|_| {
        FilesystemError::new(
            "scan_cancellation_store_unavailable",
            "The native scan cancellation store is unavailable.",
            true,
        )
    })?;
    if let Some(cancel) = scans.scans.get(&request_id) {
        cancel.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

fn resolve_scan_roots(
    root_ids: &[String],
    scan_roots: &State<'_, Mutex<ScanRootStore>>,
) -> Result<Vec<PathBuf>, FilesystemError> {
    if root_ids.is_empty() {
        return Err(FilesystemError::new(
            "scan_roots_empty",
            "Select at least one folder before scanning.",
            true,
        ));
    }

    let roots = scan_roots.lock().map_err(|_| {
        FilesystemError::new(
            "scan_root_store_unavailable",
            "The native scan-root store is unavailable.",
            true,
        )
    })?;
    let mut resolved = Vec::new();

    for id in root_ids {
        let path = roots.roots.get(id).cloned().ok_or_else(|| {
            FilesystemError::new(
                "scan_root_not_found",
                "A selected folder is no longer registered with this PC Manager session. Select it again.",
                true,
            )
        })?;
        if !resolved.contains(&path) {
            resolved.push(path);
        }
    }

    Ok(resolved)
}

fn register_scan(
    request_id: &str,
    cancellations: &State<'_, Mutex<ScanCancellationStore>>,
) -> Result<Arc<AtomicBool>, FilesystemError> {
    let mut scans = cancellations.lock().map_err(|_| {
        FilesystemError::new(
            "scan_cancellation_store_unavailable",
            "The native scan cancellation store is unavailable.",
            true,
        )
    })?;
    if scans.scans.contains_key(request_id) {
        return Err(FilesystemError::new(
            "scan_request_duplicate",
            "A filesystem scan with this request ID is already running.",
            true,
        ));
    }

    let cancel = Arc::new(AtomicBool::new(false));
    scans
        .scans
        .insert(request_id.to_string(), Arc::clone(&cancel));
    Ok(cancel)
}

fn remove_scan(request_id: &str, cancellations: &State<'_, Mutex<ScanCancellationStore>>) {
    if let Ok(mut scans) = cancellations.lock() {
        scans.scans.remove(request_id);
    }
}

fn validate_request_id(request_id: &str) -> Result<(), FilesystemError> {
    let valid = !request_id.is_empty()
        && request_id.len() <= 128
        && request_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));

    if valid {
        Ok(())
    } else {
        Err(FilesystemError::new(
            "scan_request_id_invalid",
            "The filesystem scan request identifier is invalid.",
            false,
        ))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(CleanupPlanStore::default()))
        .manage(Mutex::new(DuplicateScanPlanStore::default()))
        .manage(Mutex::new(ScanRootStore::default()))
        .manage(Mutex::new(ScanCancellationStore::default()))
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_inventory,
            run_health_check,
            scan_cleanup_candidates,
            execute_cleanup_plan,
            list_cleanup_operations,
            list_startup_entries,
            set_startup_entry_enabled,
            list_startup_operations,
            list_installed_apps,
            launch_uninstall,
            select_scan_root,
            scan_duplicates,
            delete_duplicate_files,
            scan_storage,
            cancel_filesystem_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
