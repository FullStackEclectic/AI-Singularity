use tauri::AppHandle;
use crate::services::oauth::OauthManager;

// ── Device Flow OAuth 命令 ────────────────────────────────────────────────────

/// 启动 Device Flow 授权，返回 user_code 和 verification_uri 展示给用户
#[tauri::command]
pub async fn start_oauth_flow(
    app: AppHandle,
    provider: String,
) -> Result<crate::services::oauth::DeviceFlowStartResponse, String> {
    OauthManager::start_oauth_flow(app, provider).await
}

/// 轮询授权状态（前端每 interval_seconds 秒调用一次）
/// - 返回 { done: true, token: "..." } 表示成功
/// - 返回 { done: false } 表示继续等待
/// - 返回 Err(...) 表示失败
#[tauri::command]
pub async fn poll_oauth_login(login_id: String) -> Result<serde_json::Value, String> {
    match OauthManager::poll_oauth_login(login_id).await? {
        Some(token) => Ok(serde_json::json!({ "done": true, "token": token })),
        None        => Ok(serde_json::json!({ "done": false })),
    }
}

/// 取消授权（切换 Tab 或关闭弹窗时调用）
#[tauri::command]
pub fn cancel_oauth_flow(login_id: Option<String>) -> Result<(), String> {
    OauthManager::cancel_oauth_flow(login_id)
}

/// 兼容旧接口：启动并返回 Device Flow 信息（等同于 start_oauth_flow）
#[tauri::command]
pub async fn prepare_oauth_url(
    app: AppHandle,
) -> Result<crate::services::oauth::DeviceFlowStartResponse, String> {
    OauthManager::prepare_oauth_url(app).await
}
