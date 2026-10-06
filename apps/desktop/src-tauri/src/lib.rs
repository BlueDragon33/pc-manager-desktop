use app_manager_client::{
    AgentCommandType, AppManagerError, AppManagerState, next_backoff_seconds,
};
use pc_core::{
    evaluate_health, AppInfo, AppsError, CleanupError, CleanupOperationRecord, CleanupPlanStore,
    CleanupScanOptions, CleanupScanSummary, DuplicateDeleteRequest, DuplicateDeleteResult,
    DuplicateScanOptions, DuplicateScanPlanStore, DuplicateScanSummary, FilesystemError,
    HealthReport, InstalledAppEntry, InventoryError, ScanRootSelection, StartupChangeRequest,
    StartupEntry, StartupError, StartupOperationRecord, StorageScanOptions, StorageScanSummary,
    SystemInventory, UninstallLaunchResult, UninstallRequest,
};
use pc_monitor::{MonitorError, MonitorSnapshot};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{Manager, State};

#[derive(Debug, Default)]
struct ScanRootStore {
    roots: HashMap<String, PathBuf>,
}

#[derive(Debug, Default)]
struct ScanCancellationStore {
    scans: HashMap<String, Arc<AtomicBool>>,
}

#[derive(Debug, Clone)]
struct AppManagerRuntime {
    state: Arc<Mutex<AppManagerState>>,
    syncing: Arc<AtomicBool>,
    consecutive_failures: Arc<AtomicU32>,
}

impl Default for AppManagerRuntime {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppManagerState::unconfigured())),
            syncing: Arc::new(AtomicBool::new(false)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
        }
    }
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
async fn get_monitor_snapshot() -> Result<MonitorSnapshot, MonitorError> {
    tauri::async_runtime::spawn_blocking(pc_monitor::sample_monitor)
        .await
        .map_err(|error| {
            MonitorError::new(
                "monitor_task_failed",
                format!("The monitor sampling task failed: {error}"),
                true,
            )
        })?
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
    let (plan, summary) = joined.map_err(|error| {
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

fn current_app_manager_state(runtime: &AppManagerRuntime) -> AppManagerState {
    runtime
        .state
        .lock()
        .map(|state| state.clone())
        .unwrap_or_else(|_| AppManagerState::unconfigured())
}

fn store_app_manager_state(runtime: &AppManagerRuntime, state: AppManagerState) {
    if let Ok(mut current) = runtime.state.lock() {
        *current = state;
    }
}

fn execute_app_manager_command(
    command_type: &AgentCommandType,
) -> (&'static str, serde_json::Value) {
    match command_type {
        AgentCommandType::CheckUpdate => (
            "unsupported",
            serde_json::json!({
                "reason": "Software update execution is intentionally deferred to P10."
            }),
        ),
        AgentCommandType::RunHealthScan => match pc_windows::collect_system_inventory() {
            Ok(inventory) => {
                let _ = evaluate_health(&inventory);
                (
                    "completed",
                    serde_json::json!({
                        "healthScan": "completed",
                        "detail": "A fresh local evidence-backed health scan completed."
                    }),
                )
            }
            Err(error) => (
                "failed",
                serde_json::json!({
                    "code": error.code,
                    "message": error.message,
                }),
            ),
        },
        AgentCommandType::RefreshDeviceStatus => (
            "completed",
            serde_json::json!({ "deviceStatus": "refreshed" }),
        ),
        AgentCommandType::DisableLicense => (
            "completed",
            serde_json::json!({
                "licenseDisabled": true,
                "detail": "The local entitlement state will follow App Manager policy."
            }),
        ),
    }
}

fn sync_app_manager_cycle(runtime: &AppManagerRuntime) -> AppManagerState {
    if runtime
        .syncing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return current_app_manager_state(runtime);
    }

    let outcome = app_manager_client::sync_once();
    let state = match outcome {
        Ok(batch) => {
            runtime.consecutive_failures.store(0, Ordering::Relaxed);
            let mut state = batch.state;
            let mut completed_commands = 0_u32;
            let mut command_warning: Option<String> = None;

            if let Some(device_id) = state.device_id.clone() {
                for command in batch.commands {
                    let (outcome, result) = execute_app_manager_command(&command.command_type);
                    match app_manager_client::acknowledge_command(
                        &device_id,
                        &command.command_id,
                        outcome,
                        result,
                    ) {
                        Ok(()) => {
                            completed_commands += 1;
                            if matches!(command.command_type, AgentCommandType::DisableLicense)
                                && outcome == "completed"
                            {
                                state.entitlement_state = "disabled".to_string();
                            }
                        }
                        Err(error) => {
                            command_warning = Some(format!(
                                "Command acknowledgement pending: {}",
                                error.message
                            ));
                            break;
                        }
                    }
                }
            }

            state.retry_after_seconds = batch.heartbeat_interval_seconds;
            if let Some(warning) = command_warning {
                state.message = warning;
            } else if completed_commands > 0 {
                state.message = format!(
                    "Connected to App Manager; processed {completed_commands} typed command(s)."
                );
            }
            state
        }
        Err(error) => {
            let failures = runtime
                .consecutive_failures
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            let mut state = current_app_manager_state(runtime);
            state.online = false;
            state.retry_after_seconds = next_backoff_seconds(failures);
            state.message = format!("App Manager offline: {}", error.message);
            match app_manager_client::configured_endpoint() {
                Ok(endpoint) => {
                    state.configured = endpoint.is_some();
                    state.endpoint = endpoint;
                }
                Err(config_error) => {
                    state.message = format!(
                        "App Manager configuration unavailable: {}",
                        config_error.message
                    );
                }
            }
            state
        }
    };

    store_app_manager_state(runtime, state.clone());
    runtime.syncing.store(false, Ordering::SeqCst);
    state
}

#[tauri::command]
fn get_app_manager_state(runtime: State<'_, AppManagerRuntime>) -> AppManagerState {
    current_app_manager_state(runtime.inner())
}

#[tauri::command]
async fn sync_app_manager_now(
    runtime: State<'_, AppManagerRuntime>,
) -> Result<AppManagerState, AppManagerError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || sync_app_manager_cycle(&runtime))
        .await
        .map_err(|error| {
            AppManagerError::new(
                "app_manager_sync_task_failed",
                format!("App Manager sync task failed: {error}"),
                true,
            )
        })
}

#[tauri::command]
async fn set_app_manager_endpoint(
    endpoint: Option<String>,
    runtime: State<'_, AppManagerRuntime>,
) -> Result<AppManagerState, AppManagerError> {
    app_manager_client::set_configured_endpoint(endpoint.as_deref())?;
    runtime.consecutive_failures.store(0, Ordering::Relaxed);

    let mut reset = AppManagerState::unconfigured();
    reset.configured = endpoint
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    reset.endpoint = app_manager_client::configured_endpoint()?;
    reset.message = if reset.configured {
        "App Manager endpoint saved. Connecting…".to_string()
    } else {
        "App Manager connection is disabled until an endpoint is configured.".to_string()
    };
    store_app_manager_state(runtime.inner(), reset);

    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || sync_app_manager_cycle(&runtime))
        .await
        .map_err(|error| {
            AppManagerError::new(
                "app_manager_sync_task_failed",
                format!("App Manager sync task failed: {error}"),
                true,
            )
        })
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
        .manage(AppManagerRuntime::default())
        .setup(|app| {
            let runtime = app.state::<AppManagerRuntime>().inner().clone();
            thread::spawn(move || loop {
                let state = sync_app_manager_cycle(&runtime);
                thread::sleep(Duration::from_secs(
                    state.retry_after_seconds.clamp(30, 300),
                ));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_inventory,
            get_monitor_snapshot,
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
            cancel_filesystem_scan,
            get_app_manager_state,
            sync_app_manager_now,
            set_app_manager_endpoint
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
