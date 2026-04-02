use crate::services::session_manager::{SessionManager, ChatSession, ChatMessage};

#[tauri::command]
pub fn list_sessions() -> Result<Vec<ChatSession>, String> {
    SessionManager::list_sessions()
}

#[tauri::command]
pub fn get_session_details(filepath: String) -> Result<Vec<ChatMessage>, String> {
    SessionManager::get_session_details(&filepath)
}
