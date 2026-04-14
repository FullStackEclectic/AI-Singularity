use crate::services::update_manager::{
    LinuxInstallResult, LinuxReleaseInfo, UpdateManager, UpdateRuntimeInfo, UpdateSettings,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

#[tauri::command]
pub fn get_update_settings(app: AppHandle) -> Result<UpdateSettings, String> {
    UpdateManager::load_settings(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn save_update_settings(app: AppHandle, settings: UpdateSettings) -> Result<(), String> {
    UpdateManager::save_settings(&app_data_dir(&app)?, &settings)
}

#[tauri::command]
pub fn update_last_check_time(app: AppHandle) -> Result<UpdateSettings, String> {
    UpdateManager::mark_checked_now(&app_data_dir(&app)?)
}

#[tauri::command]
pub fn get_update_runtime_info() -> Result<UpdateRuntimeInfo, String> {
    Ok(UpdateManager::runtime_info())
}

#[tauri::command]
pub async fn get_linux_update_release_info() -> Result<LinuxReleaseInfo, String> {
    UpdateManager::fetch_linux_release_info().await
}

#[tauri::command]
pub fn open_update_asset_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<String>)
        .map_err(|e| format!("打开下载链接失败: {}", e))
}

#[tauri::command]
pub async fn install_linux_update_asset(
    app: AppHandle,
    url: String,
    kind: String,
    version: Option<String>,
) -> Result<LinuxInstallResult, String> {
    UpdateManager::install_linux_release_asset(
        &app_data_dir(&app)?,
        &url,
        &kind,
        version.as_deref(),
    )
    .await
}
