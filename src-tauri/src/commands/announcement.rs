use crate::services::announcement::{AnnouncementState, AnnouncementService};
use tauri::{AppHandle, Manager};

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

#[tauri::command]
pub async fn announcement_get_state(app: AppHandle, locale: Option<String>) -> Result<AnnouncementState, String> {
    AnnouncementService::get_state(&app_data_dir(&app)?, locale.as_deref().unwrap_or("zh-CN")).await
}

#[tauri::command]
pub async fn announcement_mark_as_read(app: AppHandle, id: String) -> Result<(), String> {
    AnnouncementService::mark_as_read(&app_data_dir(&app)?, &id).await
}

#[tauri::command]
pub async fn announcement_mark_all_as_read(app: AppHandle, locale: Option<String>) -> Result<(), String> {
    AnnouncementService::mark_all_as_read(&app_data_dir(&app)?, locale.as_deref().unwrap_or("zh-CN")).await
}

#[tauri::command]
pub async fn announcement_force_refresh(app: AppHandle, locale: Option<String>) -> Result<AnnouncementState, String> {
    AnnouncementService::force_refresh(&app_data_dir(&app)?, locale.as_deref().unwrap_or("zh-CN")).await
}
