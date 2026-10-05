use pc_core::{
    evaluate_health, AppInfo, CleanupError, CleanupExecutionResult, CleanupPlanStore,
    CleanupScanOptions, CleanupScanSummary, HealthReport, InventoryError, SystemInventory,
};
use std::sync::Mutex;
use tauri::State;

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
) -> Result<CleanupExecutionResult, CleanupError> {
    let plan = {
        let stored_plans = cleanup_plans.lock().map_err(|_| {
            CleanupError::new(
                "cleanup_plan_store_unavailable",
                "The native cleanup-plan store is unavailable.",
                true,
            )
        })?;

        stored_plans.get_by_id(&plan_id).cloned().ok_or_else(|| {
            CleanupError::new(
                "cleanup_plan_not_found",
                "The requested cleanup plan is no longer available. Run a new preview scan.",
                true,
            )
        })?
    };

    pc_windows::execute_cleanup_plan(&plan)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(CleanupPlanStore::default()))
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_inventory,
            run_health_check,
            scan_cleanup_candidates,
            execute_cleanup_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
