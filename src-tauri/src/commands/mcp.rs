use crate::db::Database;
use crate::error::AppResult;
use crate::models::McpServer;
use crate::services::event_bus::EventBus;
use crate::services::mcp::McpService;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_mcps(db: State<'_, Database>) -> AppResult<Vec<McpServer>> {
    let service = McpService::new(&*db);
    service.list_mcps()
}

#[tauri::command]
pub fn add_mcp(mcp: McpServer, db: State<'_, Database>, app: AppHandle) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.add_mcp(mcp)?;
    EventBus::emit_data_changed(&app, "mcp", "add", "mcp.add");
    Ok(())
}

#[tauri::command]
pub fn toggle_mcp(id: String, is_active: bool, db: State<'_, Database>, app: AppHandle) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.toggle_mcp(&id, is_active)?;
    EventBus::emit_data_changed(&app, "mcp", "toggle", "mcp.toggle");
    Ok(())
}

#[tauri::command]
pub fn delete_mcp(id: String, db: State<'_, Database>, app: AppHandle) -> AppResult<()> {
    let service = McpService::new(&*db);
    service.delete_mcp(&id)?;
    EventBus::emit_data_changed(&app, "mcp", "delete", "mcp.delete");
    Ok(())
}
