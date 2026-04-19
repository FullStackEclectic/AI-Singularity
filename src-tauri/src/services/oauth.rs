mod config;
mod google;
mod flows;
mod polling;
mod shared;
mod trae;
mod windsurf;

use self::config::*;
use self::google::{
    build_google_auth_url, exchange_google_code, fetch_google_userinfo,
    get_google_client_secret,
};
use self::shared::{
    decode_any_jwt_claim, decode_jwt_claim, exchange_pkce_code, generate_token, sha256_b64,
    wait_for_callback,
};
use self::trae::{trae_get_login_url, trae_get_user_info};
use self::windsurf::{
    build_windsurf_auth_url, wait_for_windsurf_callback, windsurf_register_user,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthResult {
    pub token: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub meta_json: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub provider: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceFlowStartResponse {
    pub login_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval_seconds: u64,
}

#[derive(Debug)]
struct OAuthSession {
    provider: String,
    #[allow(dead_code)]
    callback_port: Option<u16>,
    pending_result: Option<OAuthResult>,
    state_token: Option<String>,
    code_verifier: Option<String>,
    device_code: Option<String>,
    cancel_tx: watch::Sender<bool>,
    expires_at: Instant,
}

static SESSION_MAP: Mutex<Option<HashMap<String, OAuthSession>>> = Mutex::new(None);

fn with_sessions<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, OAuthSession>) -> R,
{
    let mut guard = SESSION_MAP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

pub struct OauthManager;

impl OauthManager {
    pub async fn start_oauth_flow(
        app: tauri::AppHandle,
        provider: String,
    ) -> Result<DeviceFlowStartResponse, String> {
        if is_import_only_provider(&provider) {
            return Err(format!(
                "「{}」渠道不支持 OAuth 授权，请切换到「Token 粘贴」或「本地导入」Tab",
                provider
            ));
        }

        if is_device_flow_provider(&provider) {
            return Self::start_github_device_flow().await;
        }

        if is_cursor_provider(&provider) {
            return Self::start_cursor_flow();
        }

        if is_server_poll_provider(&provider) {
            return Self::start_server_poll_flow(app, &provider).await;
        }

        if is_localhost_redirect_provider(&provider) {
            return Self::start_localhost_redirect_flow(app, &provider).await;
        }

        Err(format!("未知的 provider: {}", provider))
    }

}
