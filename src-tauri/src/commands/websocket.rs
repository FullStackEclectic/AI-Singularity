use crate::services::websocket::WebSocketStatus;

#[tauri::command]
pub fn get_websocket_status() -> Result<WebSocketStatus, String> {
    Ok(crate::services::websocket::get_status())
}
