use crate::services::ide_scanner::{IdeScanner, ScannedIdeAccount};

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
