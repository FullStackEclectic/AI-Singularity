use crate::error::AppResult;
use crate::models::McpServer;
use crate::services::mcp::McpService;
use tauri::State;
use crate::db::Database;

#[tauri::command]
pub fn get_mcps(db: State<'_, Database>) -> AppResult<Vec<McpServer>> {
    let service = McpService::new(&*db);
    service.list_mcps()
}

#[tauri::command]
pub fn add_mcp(mcp: McpServer, db: State<'_, Database>) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.add_mcp(mcp)
}

#[tauri::command]
pub fn toggle_mcp(id: String, is_active: bool, db: State<'_, Database>) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.toggle_mcp(&id, is_active)
}

#[tauri::command]
pub fn delete_mcp(id: String, db: State<'_, Database>) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.delete_mcp(&id)
}
