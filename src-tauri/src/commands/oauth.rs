use tauri::AppHandle;
use crate::services::oauth::{OauthManager, DeviceFlowStartResponse};

// ── OAuth 命令层 ─────────────────────────────────────────────────────────────
//
// 命令名保持不变，前端无需改动。
// 底层实现已重构为多渠道路由架构（见 services/oauth.rs）。

/// 启动 OAuth 授权流程，按 provider 自动路由到对应实现：
/// - A 类（antigravity / gemini）：浏览器回调型，打开浏览器后自动监听回调
/// - B 类（github_copilot）：Device Flow，返回 user_code + verification_uri
/// - Cursor：DeepControl 轮询型
/// - 文件导入型（claude_code / vscode 等）：直接返回错误提示用户切换 Tab
#[tauri::command]
pub async fn start_oauth_flow(
    app: AppHandle,
    provider: String,
) -> Result<DeviceFlowStartResponse, String> {
    OauthManager::start_oauth_flow(app, provider).await
}

/// 轮询授权状态（前端每 interval_seconds 秒调用一次）
///
/// 返回 JSON：
/// - `{ "done": true, "token": "...", "email": "...", "name": "..." }` → 授权成功
/// - `{ "done": false }`                                               → 继续等待
/// - Err(msg)                                                           → 失败/超时/取消
#[tauri::command]
pub async fn poll_oauth_login(login_id: String) -> Result<serde_json::Value, String> {
    match OauthManager::poll_oauth_login(login_id).await? {
        Some(result) => Ok(serde_json::json!({
            "done":  true,
            "token": result.token,
            "email": result.email,
            "name":  result.name,
        })),
        None => Ok(serde_json::json!({ "done": false })),
    }
}

/// 取消授权（切换 Tab 或关闭弹窗时调用）
/// login_id 为 None 时取消所有进行中的流程
#[tauri::command]
pub fn cancel_oauth_flow(login_id: Option<String>) -> Result<(), String> {
    OauthManager::cancel_oauth_flow(login_id)
}

/// 预生成授权 URL（兼容接口）
/// 等同于 start_oauth_flow("antigravity")
#[tauri::command]
pub async fn prepare_oauth_url(app: AppHandle) -> Result<DeviceFlowStartResponse, String> {
    OauthManager::prepare_oauth_url(app).await
}
