use crate::services::logs::{DesktopLogFile, DesktopLogReadResult, LogsService};
use tauri::{AppHandle, Manager};

fn logs_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))?
        .join("logs"))
}

#[tauri::command]
pub fn list_desktop_logs(app: AppHandle) -> Result<Vec<DesktopLogFile>, String> {
    LogsService::list_logs(&logs_dir(&app)?)
}

#[tauri::command]
pub fn read_desktop_log(
    app: AppHandle,
    name: String,
    lines: Option<usize>,
    query: Option<String>,
) -> Result<DesktopLogReadResult, String> {
    LogsService::read_log(
        &logs_dir(&app)?,
        &name,
        lines.unwrap_or(500),
        query.as_deref(),
    )
}

#[tauri::command]
pub fn export_desktop_log(app: AppHandle, name: String, destination: String) -> Result<(), String> {
    LogsService::export_log(&logs_dir(&app)?, &name, std::path::Path::new(&destination))
}
