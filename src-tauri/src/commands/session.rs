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

#[tauri::command]
pub fn launch_session_terminal(cwd: String, command: String) -> Result<(), String> {
    // Attempt to launch wt.exe, fallback to cmd.exe
    // using cmd.exe /c start cmd.exe /K "cd /d cwd && command" to keep it open
    #[cfg(target_os = "windows")]
    {
        let cmd_str = format!("cd /d \"{}\" && {}", cwd, command);
        // We use cmd.exe to start the new window.
        match std::process::Command::new("cmd.exe")
            .args(&["/C", "start", "cmd.exe", "/K", &cmd_str])
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to launch terminal: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("One-click terminal launch is currently only supported on Windows.".into())
    }
}
