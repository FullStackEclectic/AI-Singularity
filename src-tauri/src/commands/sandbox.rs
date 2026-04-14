use crate::error::AppResult;
use crate::services::sandbox::SandboxManager;

#[tauri::command]
pub async fn launch_tool_sandboxed(command_str: String, proxy_port: u16) -> AppResult<()> {
    SandboxManager::launch_tool_sandboxed(&command_str, proxy_port)
}
