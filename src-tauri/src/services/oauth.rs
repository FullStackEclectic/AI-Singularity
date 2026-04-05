use tauri::AppHandle;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

// ── Device Flow 会话结构 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DeviceFlowStartResponse {
    pub login_id:          String,
    pub user_code:         String,
    pub verification_uri:  String,
    pub expires_in:        u64,
    pub interval_seconds:  u64,
}

#[derive(Debug, Clone)]
struct DeviceFlowSession {
    device_code:   String,
    expires_at:    Instant,
    cancelled:     bool,
}

// ── GitHub Device Flow API 响应 ───────────────────────────────────────────────

#[derive(Deserialize)]
struct GhDeviceCodeResponse {
    device_code:      String,
    user_code:        String,
    verification_uri: String,
    expires_in:       u64,
    interval:         u64,
}

#[derive(Deserialize)]
struct GhTokenResponse {
    access_token: Option<String>,
    error:        Option<String>,
}

// ── 全局 Session 注册表 ───────────────────────────────────────────────────────

static SESSION_MAP: Mutex<Option<HashMap<String, DeviceFlowSession>>> = Mutex::new(None);

fn with_sessions<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, DeviceFlowSession>) -> R,
{
    let mut guard = SESSION_MAP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

// ── OauthManager ─────────────────────────────────────────────────────────────

pub struct OauthManager;

impl OauthManager {
    /// 启动 GitHub Device Flow 授权（异步）
    pub async fn start_oauth_flow(
        _app: AppHandle,
        _provider: String,
    ) -> Result<DeviceFlowStartResponse, String> {
        // GitHub OAuth App Client ID（需替换为实际申请的）
        let client_id = "Iv23liRzHiTiFiMGb9nd";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端构建失败: {}", e))?;

        let resp = client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&[("client_id", client_id), ("scope", "read:user")])
            .send()
            .await
            .map_err(|e| format!("请求 Device Code 失败（请检查网络连接）: {}", e))?;

        let body: GhDeviceCodeResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析 Device Code 响应失败: {}", e))?;

        let login_id = uuid::Uuid::new_v4().to_string();
        let session = DeviceFlowSession {
            device_code:   body.device_code.clone(),
            expires_at:    Instant::now() + Duration::from_secs(body.expires_in),
            cancelled:     false,
        };

        with_sessions(|map| map.insert(login_id.clone(), session));

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code:        body.user_code,
            verification_uri: body.verification_uri,
            expires_in:       body.expires_in,
            interval_seconds: body.interval,
        })
    }

    /// 轮询授权状态（前端每 interval_seconds 调用一次，异步）
    /// - Ok(Some(token)) → 成功
    /// - Ok(None)        → 继续等待
    /// - Err(msg)        → 失败/超时/取消
    pub async fn poll_oauth_login(login_id: String) -> Result<Option<String>, String> {
        let (device_code, expired, cancelled) = with_sessions(|map| {
            if let Some(s) = map.get(&login_id) {
                (
                    s.device_code.clone(),
                    s.expires_at < Instant::now(),
                    s.cancelled,
                )
            } else {
                ("".to_string(), true, true)
            }
        });

        if cancelled {
            return Err("授权已取消".to_string());
        }
        if expired {
            with_sessions(|map| map.remove(&login_id));
            return Err("授权码已过期，请重新启动授权".to_string());
        }

        let client_id = "Iv23liRzHiTiFiMGb9nd";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端构建失败: {}", e))?;

        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| format!("轮询请求失败: {}", e))?;

        let token_resp: GhTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析轮询响应失败: {}", e))?;

        if let Some(token) = token_resp.access_token {
            if !token.is_empty() {
                with_sessions(|map| map.remove(&login_id));
                return Ok(Some(token));
            }
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") | Some("slow_down") | None => Ok(None),
            Some("expired_token") => {
                with_sessions(|map| map.remove(&login_id));
                Err("授权码已过期".to_string())
            }
            Some("access_denied") => {
                with_sessions(|map| map.remove(&login_id));
                Err("用户拒绝了授权请求".to_string())
            }
            Some(other) => {
                with_sessions(|map| map.remove(&login_id));
                Err(format!("授权失败: {}", other))
            }
        }
    }

    /// 取消授权
    pub fn cancel_oauth_flow(login_id: Option<String>) -> Result<(), String> {
        with_sessions(|map| {
            match &login_id {
                Some(id) => { map.remove(id); }
                None     => { map.clear(); }
            }
        });
        Ok(())
    }

    /// 等同于 start_oauth_flow（Antigravity-Manager 兼容接口）
    pub async fn prepare_oauth_url(app: AppHandle) -> Result<DeviceFlowStartResponse, String> {
        Self::start_oauth_flow(app, "antigravity".to_string()).await
    }
}
