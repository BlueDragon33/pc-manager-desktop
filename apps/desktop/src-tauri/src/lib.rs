use pc_core::{evaluate_health, AppInfo, HealthReport, InventoryError, SystemInventory};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_inventory,
            run_health_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
