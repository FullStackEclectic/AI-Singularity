use crate::services::session_manager::{SessionManager, ChatSession, ChatMessage, ZombieProcess};

#[tauri::command]
pub fn list_sessions() -> Result<Vec<ChatSession>, String> {
    SessionManager::list_sessions()
}

#[tauri::command]
pub fn get_session_details(filepath: String) -> Result<Vec<ChatMessage>, String> {
    SessionManager::get_session_details(&filepath)
}

#[tauri::command]
pub fn scan_zombies() -> Vec<ZombieProcess> {
    SessionManager::scan_zombie_processes()
}
