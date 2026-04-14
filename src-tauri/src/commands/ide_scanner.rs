use crate::services::ide_scanner::{FileImportScanResult, IdeScanner, ScannedIdeAccount};

/// 自动扫描本机常见 IDE 路径，提取账号
#[tauri::command]
pub fn scan_ide_accounts_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::scan_ide_accounts_from_local()
}

/// 从用户指定的 .vscdb 文件提取账号
#[tauri::command]
pub fn import_from_custom_db(path: String) -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_from_custom_db(path)
}

/// 迁移旧版 v1 格式账号
#[tauri::command]
pub fn import_v1_accounts() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_v1_accounts()
}

#[tauri::command]
pub fn import_from_files(paths: Vec<String>) -> Result<FileImportScanResult, String> {
    IdeScanner::import_from_files(paths)
}

#[tauri::command]
pub fn import_gemini_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_gemini_from_local()
}

#[tauri::command]
pub fn import_codex_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_codex_from_local()
}

#[tauri::command]
pub fn import_kiro_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_kiro_from_local()
}

#[tauri::command]
pub fn import_cursor_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_cursor_from_local()
}

#[tauri::command]
pub fn import_windsurf_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_windsurf_from_local()
}

#[tauri::command]
pub fn import_codebuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_codebuddy_from_local()
}

#[tauri::command]
pub fn import_codebuddy_cn_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_codebuddy_cn_from_local()
}

#[tauri::command]
pub fn import_workbuddy_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_workbuddy_from_local()
}

#[tauri::command]
pub fn import_zed_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_zed_from_local()
}

#[tauri::command]
pub fn import_qoder_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_qoder_from_local()
}

#[tauri::command]
pub fn import_trae_from_local() -> Result<Vec<ScannedIdeAccount>, String> {
    IdeScanner::import_trae_from_local()
}
