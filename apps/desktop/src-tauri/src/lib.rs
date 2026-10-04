use pc_core::{
    evaluate_health, AppInfo, CleanupError, CleanupPlan, CleanupScanOptions, CleanupScanSummary,
    HealthReport, InventoryError, SystemInventory,
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
    cleanup_plan: State<'_, Mutex<Option<CleanupPlan>>>,
) -> Result<CleanupScanSummary, CleanupError> {
    let (plan, summary) = pc_windows::scan_cleanup_candidates(options)?;
    let mut stored_plan = cleanup_plan.lock().map_err(|_| {
        CleanupError::new(
            "cleanup_plan_store_unavailable",
            "The native cleanup-plan store is unavailable.",
            true,
        )
    })?;

    *stored_plan = Some(plan);
    Ok(summary)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(None::<CleanupPlan>))
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_inventory,
            run_health_check,
            scan_cleanup_candidates
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
