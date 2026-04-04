use crate::error::AppResult;
use crate::services::provisioner::{ProvisionerManager, ToolStatus};
use tauri::Window;

#[tauri::command]
pub async fn check_tool_status(tool_id: String) -> AppResult<ToolStatus> {
    ProvisionerManager::check_status(&tool_id)
}

#[tauri::command]
pub async fn deploy_tool(tool_id: String, window: Window) -> AppResult<()> {
    // Note: this will block the async runtime if we don't spawn a blocking task, 
    // but Tauri command runner will handle it as futures if we don't thread block too heavily,
    // actually, std::process::Child::wait is blocking, so let's wrap it in spawn_blocking.
    
    let win_clone = window.clone();
    tokio::task::spawn_blocking(move || {
        ProvisionerManager::deploy_tool(&tool_id, win_clone)
    })
    .await
    .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))??;
    
    Ok(())
}
