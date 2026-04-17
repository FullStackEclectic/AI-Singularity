use tauri::AppHandle;

#[tauri::command]
pub fn tray_get_platform_scope(app: AppHandle) -> Result<Vec<String>, String> {
    Ok(crate::tray::get_tray_platform_scope(&app))
}

#[tauri::command]
pub fn tray_set_platform_scope(
    app: AppHandle,
    platforms: Vec<String>,
) -> Result<Vec<String>, String> {
    crate::tray::set_tray_platform_scope(&app, platforms)
}
