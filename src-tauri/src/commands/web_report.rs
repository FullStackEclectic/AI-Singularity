#[tauri::command]
pub fn get_web_report_port() -> Result<Option<u16>, String> {
    Ok(crate::services::web_report::get_port())
}

#[tauri::command]
pub fn get_web_report_status() -> Result<crate::services::web_report::WebReportStatus, String> {
    Ok(crate::services::web_report::get_status())
}
