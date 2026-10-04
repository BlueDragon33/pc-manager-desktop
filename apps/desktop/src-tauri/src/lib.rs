use pc_core::{AppInfo, InventoryError, SystemInventory};

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo::current()
}

#[tauri::command]
fn get_system_inventory() -> Result<SystemInventory, InventoryError> {
    pc_windows::collect_system_inventory()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_info, get_system_inventory])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
