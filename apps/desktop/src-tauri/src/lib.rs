use pc_core::AppInfo;

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo::p0()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running PC Manager Desktop");
}
