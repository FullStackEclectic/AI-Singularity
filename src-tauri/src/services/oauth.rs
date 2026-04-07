use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};
use url::Url;
use base64::Engine as _;

// ── 常量配置 ─────────────────────────────────────────────────────────────────

// Antigravity（Google OAuth）
const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const ANTIGRAVITY_CLIENT_SECRET: &str = "TODO_REPLACE_WITH_OAUTH_SECRET";

// Gemini CLI（Google OAuth）
const GEMINI_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const GEMINI_CLIENT_SECRET: &str = "TODO_REPLACE_WITH_GEMINI_SECRET";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const GOOGLE_OAUTH_CALLBACK_PATH: &str = "/oauth2callback";
const GOOGLE_SCOPES: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile";

// Cursor DeepControl
const CURSOR_LOGIN_URL: &str = "https://cursor.com/loginDeepControl";
const CURSOR_POLL_URL: &str = "https://api2.cursor.sh/auth/poll";

// GitHub Copilot Device Flow
const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_CLIENT_ID: &str = "01ab8ac9400c4e429b23";
const GITHUB_SCOPE: &str = "read:user user:email";

// Windsurf OAuth
const WINDSURF_CLIENT_ID: &str = "3GUryQ7ldAeKEuD2obYnppsnmj58eP5u";
const WINDSURF_AUTH_BASE_URL: &str = "https://www.windsurf.com";
const WINDSURF_REGISTER_API_URL: &str = "https://register.windsurf.com";
const WINDSURF_DEFAULT_API_SERVER: &str = "https://server.codeium.com";
const WINDSURF_CALLBACK_PATH: &str = "/windsurf-auth-callback";

// Codex (OpenAI Codex) OAuth - PKCE
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_SCOPES: &str = "openid profile email offline_access";
const CODEX_CALLBACK_PATH: &str = "/auth/callback";
const CODEX_CALLBACK_PORT: u16 = 1455;

// Kiro (Amazon Q) OAuth - PKCE
const KIRO_AUTH_URL: &str = "https://app.kiro.dev/signin";
const KIRO_TOKEN_URL: &str = "https://prod.us-east-1.auth.desktop.kiro.dev/oauth/token";
const KIRO_CALLBACK_PATH: &str = "/oauth/callback";

// Trae OAuth
const TRAE_AUTH_CLIENT_ID: &str = "ono9krqynydwx5";
const TRAE_CALLBACK_PATH: &str = "/authorize";
const TRAE_LOGIN_GUIDANCE_URL: &str = "https://api.marscode.com/cloudide/api/v3/trae/GetLoginGuidance";
const TRAE_EXCHANGE_TOKEN_PATH: &str = "/cloudide/api/v3/trae/oauth/ExchangeToken";

// Qoder OAuth - 服务端轮询
const QODER_LOGIN_URL: &str = "https://qoder.com/device/selectAccounts";
const QODER_OPENAPI_URL: &str = "https://openapi.qoder.sh";
const QODER_CLIENT_ID: &str = "e883ade2-e6e3-4d6d-adf7-f92ceff5fdcb";
const QODER_POLL_PATH: &str = "/api/v1/deviceToken/poll";
const QODER_USERINFO_PATH: &str = "/api/v1/userinfo";

// CodeBuddy OAuth - 服务端 state 轮询
const CODEBUDDY_API_URL: &str = "https://www.codebuddy.ai";
const CODEBUDDY_API_PREFIX: &str = "/v2/plugin";

// Zed OAuth - RSA 加密回调
const ZED_SIGNIN_URL: &str = "https://zed.dev/native_app_signin";
const ZED_CALLBACK_PATH: &str = "/zed-auth-callback";

// APP 超时
const OAUTH_TIMEOUT_SECS: u64 = 300;


// ── 渠道分类 ──────────────────────────────────────────────────────────────────

/// A 类：浏览器回调型（Localhost Redirect）
fn is_localhost_redirect_provider(provider: &str) -> bool {
    matches!(
        provider,
        "antigravity" | "gemini" | "windsurf" | "zed" | "codex" | "kiro" | "trae"
    )
}

/// B 类：Device Flow 轮询型
fn is_device_flow_provider(provider: &str) -> bool {
    matches!(provider, "github_copilot")
}

/// Cursor DeepControl 轮询型（特殊）
fn is_cursor_provider(provider: &str) -> bool {
    matches!(provider, "cursor")
}

/// D 类：服务端 state 轮询型（无 TCP 回调）
fn is_server_poll_provider(provider: &str) -> bool {
    matches!(provider, "qoder" | "codebuddy")
}

/// 只能文件导入，不支持 OAuth
fn is_import_only_provider(provider: &str) -> bool {
    matches!(
        provider,
        "claude_code" | "claude_desktop" | "vscode" | "opencode" | "generic_ide"
    )
}

// ── 公共数据结构 ──────────────────────────────────────────────────────────────

/// 授权完成后的结构化结果（存入 SESSION_MAP，轮询时返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthResult {
    /// refresh_token 优先，否则为 access_token
    pub token:    String,
    pub email:    Option<String>,
    pub name:     Option<String>,
    pub provider: String,
    /// 发生错误时非空
    pub error:    Option<String>,
}

/// 启动 OAuth 流程的返回结构（前端 `DeviceFlowStart` 接口）
#[derive(Debug, Clone, Serialize)]
pub struct DeviceFlowStartResponse {
    pub login_id:         String,
    /// Device Flow 的 6-8 位验证码（其它渠道为空字符串）
    pub user_code:        String,
    /// 授权链接（用户需要访问的 URL）
    pub verification_uri: String,
    /// 到期时间（秒）
    pub expires_in:       u64,
    /// 前端应该多久轮询一次
    pub interval_seconds: u64,
}

// ── Session 内部状态 ──────────────────────────────────────────────────────────

#[derive(Debug)]
struct OAuthSession {
    provider:       String,
    callback_port:  Option<u16>,
    /// 授权完成后写入（A类/Cursor/GitHub 共用 None 表示等待中）
    pending_result: Option<OAuthResult>,
    state_token:    Option<String>,
    code_verifier:  Option<String>,
    device_code:    Option<String>,
    cancel_tx:      watch::Sender<bool>,
    expires_at:     Instant,
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

// ── Google OAuth 工具函数 ─────────────────────────────────────────────────────

fn build_google_auth_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    let mut url = Url::parse(GOOGLE_AUTH_URL).unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("access_type", "offline");
        q.append_pair("scope", GOOGLE_SCOPES);
        q.append_pair("state", state);
        q.append_pair("prompt", "consent"); // 强制返回 refresh_token
    }
    url.to_string()
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token:      Option<String>,
    refresh_token:     Option<String>,
    error:             Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: Option<String>,
    name:  Option<String>,
}

async fn exchange_google_code(
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("换取 Google token 失败: {}", e))?;

    let body: GoogleTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 Google token 响应失败: {}", e))?;

    if let Some(err) = body.error {
        return Err(format!(
            "Google 授权失败: {} ({})",
            err,
            body.error_description.unwrap_or_default()
        ));
    }

    let access_token = body.access_token.ok_or("Google token 响应缺少 access_token")?;
    Ok((access_token, body.refresh_token))
}

async fn fetch_google_userinfo(access_token: &str) -> Option<GoogleUserInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .ok()?;
    resp.json::<GoogleUserInfo>().await.ok()
}

// ── 通用工具 ──────────────────────────────────────────────────────────────────

fn generate_token(len: usize) -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..len)
        .map(|_| rand::thread_rng().gen::<u8>())
        .collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn sha256_b64(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(h.finalize().as_slice())
}

// ── Windsurf 工具函数 ─────────────────────────────────────────────────────────

/// 构建 Windsurf OAuth URL（Implicit Flow，response_type=token）
fn build_windsurf_auth_url(redirect_uri: &str, state: &str) -> String {
    let mut url = Url::parse(&format!("{}/windsurf/signin", WINDSURF_AUTH_BASE_URL)).unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "token");
        q.append_pair("client_id", WINDSURF_CLIENT_ID);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("state", state);
        q.append_pair("prompt", "login");
        q.append_pair("redirect_parameters_type", "query");
        q.append_pair("workflow", "onboarding");
    }
    url.to_string()
}

/// 等待 Windsurf 回调（Implicit Flow，access_token 直接在 query 参数中）
/// 成功返回 (access_token, state)
async fn wait_for_windsurf_callback(
    port: u16,
    expected_state: String,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定回调端口 {} 失败: {}", port, e))?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (mut stream, _) = res.map_err(|e| format!("接受连接失败: {}", e))?;

                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                let params = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|path| Url::parse(&format!("http://127.0.0.1:{}{}", port, path)).ok())
                    .map(|url| url.query_pairs()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<HashMap<String, String>>())
                    .unwrap_or_default();

                let received_state = params.get("state").cloned().unwrap_or_default();
                let access_token  = params.get("access_token").cloned().unwrap_or_default();
                let error         = params.get("error").cloned();

                if received_state != expected_state {
                    let _ = stream.write_all(CALLBACK_FAIL_HTML.as_bytes()).await;
                    continue;
                }

                if let Some(err) = error {
                    let _ = stream.write_all(CALLBACK_FAIL_HTML.as_bytes()).await;
                    return Err(format!("Windsurf 授权拒绝: {}", err));
                }

                if access_token.is_empty() {
                    let _ = stream.write_all(CALLBACK_FAIL_HTML.as_bytes()).await;
                    continue;
                }

                let _ = stream.write_all(CALLBACK_SUCCESS_HTML.as_bytes()).await;
                return Ok(access_token);
            }
            _ = cancel_rx.changed() => {
                return Err("授权已取消".to_string());
            }
            _ = tokio::time::sleep(Duration::from_secs(OAUTH_TIMEOUT_SECS)) => {
                return Err("等待授权超时，请重试".to_string());
            }
        }
    }
}

/// 用 Firebase ID Token（即 access_token）注册用户，换取 api_key 和 email
async fn windsurf_register_user(
    access_token: &str,
) -> Result<(String, Option<String>, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let body = serde_json::json!({ "firebase_id_token": access_token });
    let url = format!(
        "{}/exa.seat_management_pb.SeatManagementService/RegisterUser",
        WINDSURF_REGISTER_API_URL
    );

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "ai-singularity")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Windsurf RegisterUser 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Windsurf RegisterUser 失败: HTTP {} — {}", status, text));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Windsurf RegisterUser 响应失败: {}", e))?;

    let api_key = value.get("apiKey")
        .or_else(|| value.get("api_key"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or("Windsurf RegisterUser 响应缺少 apiKey")?;

    let name = value.get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    // 尝试从 api_server_url 分支获取 email（可能在 GetCurrentUser 里，这里用 api_key 替代主 token）
    // 先返回 api_key 和 name，email 留空（Windsurf 的 email 需要二次 GetCurrentUser 请求）
    Ok((api_key, name, None))
}

// ── 通用 PKCE code exchange ───────────────────────────────────────────────────

/// 通用 PKCE code exchange，发 POST 到 token_endpoint
async fn exchange_pkce_code(
    token_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];
    if !client_id.is_empty() {
        params.push(("client_id", client_id));
    }

    let resp = client
        .post(token_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("PKCE token exchange 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("PKCE token exchange 失败: HTTP {} — {}", st, body));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析 PKCE token 响应失败: {}", e))
}

/// 从 JWT access_token payload 提取单个声明字段
fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let padding = (4 - parts[1].len() % 4) % 4;
    let padded = format!("{}{}", parts[1], "=".repeat(padding));
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&padded).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    payload.get(claim)?.as_str().filter(|s| !s.is_empty()).map(String::from)
}

// ── Trae 工具函数 ─────────────────────────────────────────────────────────────

/// 请求 Trae GetLoginGuidance，获取真实的登录 URL
async fn trae_get_login_url(redirect_uri: &str, state: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let body = serde_json::json!({
        "client_id": TRAE_AUTH_CLIENT_ID,
        "redirect_uri": redirect_uri,
        "state": state
    });

    let resp = client
        .post(TRAE_LOGIN_GUIDANCE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Trae GetLoginGuidance 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let st = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Trae GetLoginGuidance 失败: HTTP {} — {}", st, text));
    }

    let value: serde_json::Value = resp.json().await
        .map_err(|e| format!("解析 Trae GetLoginGuidance 响应失败: {}", e))?;

    // 尝试多个字段路径
    value.get("data")
        .and_then(|d| d.get("authUrl").or_else(|| d.get("auth_url")).or_else(|| d.get("url")))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            // 兜底：构建标准 URL
            Some(format!(
                "https://www.trae.ai/oauth/authorization?client_id={}&redirect_uri={}&state={}",
                TRAE_AUTH_CLIENT_ID,
                urlencoding::encode(redirect_uri),
                state
            ))
        })
        .ok_or("无法获取 Trae 登录 URL".to_string())
}

/// 通过 Trae refresh_token 获取用户信息（email/name）
async fn trae_get_user_info(token: &str, login_host: &str) -> Result<(Option<String>, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    // 构建 GetUserInfo URL
    let base = if login_host.starts_with("http") {
        login_host.trim_end_matches('/').to_string()
    } else {
        "https://api.marscode.com".to_string()
    };
    let url = format!("{}{}", base, "/cloudide/api/v3/trae/GetUserInfo");

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("Trae GetUserInfo 请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Ok((None, None));
    }

    let value: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    let data = value.get("data").unwrap_or(&serde_json::Value::Null);
    let email = data.get("email").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    let name = data.get("name").or_else(|| data.get("nickname"))
        .and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    Ok((email, name))
}

// ── 本地 TCP 回调服务器（Google） ─────────────────────────────────────────────


static CALLBACK_SUCCESS_HTML: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<html><body style='font-family:sans-serif;background:#0f172a;color:#e2e8f0;\
padding:32px;text-align:center;'>\
<h2 style='color:#22c55e;'>✅ 授权成功</h2>\
<p>可以关闭此窗口并返回 AI Singularity。</p>\
<script>setTimeout(function(){window.close();},1500);</script>\
</body></html>";

static CALLBACK_FAIL_HTML: &str = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<html><body style='font-family:sans-serif;background:#0f172a;color:#e2e8f0;\
padding:32px;text-align:center;'>\
<h2 style='color:#ef4444;'>❌ 授权失败</h2>\
<p>state 校验失败或回调参数缺失，请重新尝试。</p>\
</body></html>";

/// 启动本地 TCP 监听，等待 OAuth 回调
/// 成功返回 (authorization_code, redirect_uri)
async fn wait_for_callback(
    port: u16,
    expected_state: String,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<(String, String), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定回调端口 {} 失败: {}", port, e))?;

    let redirect_uri = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (mut stream, _) = res.map_err(|e| format!("接受连接失败: {}", e))?;

                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                // 解析请求行中的 URL
                let params = request
                    .lines()
                    .next()
                    .and_then(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        parts.get(1).copied()
                    })
                    .and_then(|path| {
                        Url::parse(&format!("http://127.0.0.1:{}{}", port, path)).ok()
                    })
                    .map(|url| {
                        url.query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect::<HashMap<String, String>>()
                    })
                    .unwrap_or_default();

                let received_state = params.get("state").cloned().unwrap_or_default();
                let code = params.get("code").cloned().unwrap_or_default();

                if received_state != expected_state || code.is_empty() {
                    let _ = stream.write_all(CALLBACK_FAIL_HTML.as_bytes()).await;
                    continue;
                }

                let _ = stream.write_all(CALLBACK_SUCCESS_HTML.as_bytes()).await;
                return Ok((code, redirect_uri));
            }
            _ = cancel_rx.changed() => {
                return Err("授权已取消".to_string());
            }
            _ = tokio::time::sleep(Duration::from_secs(OAUTH_TIMEOUT_SECS)) => {
                return Err("等待授权超时，请重试".to_string());
            }
        }
    }
}

// ── OauthManager 对外接口 ─────────────────────────────────────────────────────

pub struct OauthManager;

impl OauthManager {
    /// 启动 OAuth 流程，按 provider 路由到不同实现
    pub async fn start_oauth_flow(
        app: tauri::AppHandle,
        provider: String,
    ) -> Result<DeviceFlowStartResponse, String> {
        // 文件导入类
        if is_import_only_provider(&provider) {
            return Err(format!(
                "「{}」渠道不支持 OAuth 授权，请切换到「Token 粘贴」或「本地导入」Tab",
                provider
            ));
        }

        // GitHub Copilot Device Flow
        if is_device_flow_provider(&provider) {
            return Self::start_github_device_flow().await;
        }

        // Cursor DeepControl
        if is_cursor_provider(&provider) {
            return Self::start_cursor_flow();
        }

        // D 类：服务端 state 轮询（qoder/codebuddy）
        if is_server_poll_provider(&provider) {
            return Self::start_server_poll_flow(app, &provider).await;
        }

        // A 类：浏览器回调型
        if is_localhost_redirect_provider(&provider) {
            return Self::start_localhost_redirect_flow(app, &provider).await;
        }

        Err(format!("未知的 provider: {}", provider))
    }

    // ── A 类：Localhost Redirect ──────────────────────────────────────────────

    async fn start_localhost_redirect_flow(
        app: tauri::AppHandle,
        provider: &str,
    ) -> Result<DeviceFlowStartResponse, String> {
        // 找可用端口
        let probe = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("绑定本地端口失败: {}", e))?;
        let port = probe.local_addr()
            .map_err(|e| format!("获取本地端口失败: {}", e))?
            .port();
        drop(probe);

        let state_token = generate_token(24);
        let login_id    = generate_token(16);

        // 按渠道区分 OAuth URL 构建方式
        let (auth_url, redirect_uri, extra_state) = match provider {
            "antigravity" => {
                let redir = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);
                let url   = build_google_auth_url(ANTIGRAVITY_CLIENT_ID, &redir, &state_token);
                (url, redir, None::<String>)
            }
            "gemini" => {
                let redir = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);
                let url   = build_google_auth_url(GEMINI_CLIENT_ID, &redir, &state_token);
                (url, redir, None)
            }
            "windsurf" => {
                let redir = format!("http://127.0.0.1:{}{}", port, WINDSURF_CALLBACK_PATH);
                let url   = build_windsurf_auth_url(&redir, &state_token);
                (url, redir, None)
            }
            "codex" => {
                // Codex 固定使用 port 1455
                let redir = format!("http://localhost:{}{}", CODEX_CALLBACK_PORT, CODEX_CALLBACK_PATH);
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let url = format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&originator=codex_vscode&codex_cli_simplified_flow=true",
                    CODEX_AUTH_URL,
                    CODEX_CLIENT_ID,
                    urlencoding::encode(&redir),
                    urlencoding::encode(CODEX_SCOPES),
                    code_challenge,
                    state_token
                );
                (url, redir, Some(code_verifier))
            }
            "kiro" => {
                let redir = format!("http://127.0.0.1:{}{}", port, KIRO_CALLBACK_PATH);
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let url = format!(
                    "{}?state={}&code_challenge={}&code_challenge_method=S256&redirect_uri={}&redirect_from=KiroIDE",
                    KIRO_AUTH_URL,
                    urlencoding::encode(&state_token),
                    urlencoding::encode(&code_challenge),
                    urlencoding::encode(&redir)
                );
                (url, redir, Some(code_verifier))
            }
            "trae" => {
                // Trae 需要先请求 GetLoginGuidance 获取登录地址
                let redir = format!("http://127.0.0.1:{}{}", port, TRAE_CALLBACK_PATH);
                // 构建一个临时 URL，后台任务里会替换
                let url = format!(
                    "https://www.trae.ai/oauth/authorize?client_id={}&redirect_uri={}&state={}",
                    TRAE_AUTH_CLIENT_ID,
                    urlencoding::encode(&redir),
                    state_token
                );
                (url, redir, None)
            }
            "zed" => {
                let redir = format!("http://127.0.0.1:{}{}", port, ZED_CALLBACK_PATH);
                // Zed 需要 RSA 密钥对，这里先构建基础 URL，密钥在后台任务里生成
                let url = format!(
                    "{}?app_callback_url={}&state={}",
                    ZED_SIGNIN_URL,
                    urlencoding::encode(&redir),
                    state_token
                );
                (url, redir, None)
            }
            _ => return Err(format!(
                "「{}」OAuth 暂未支持，敬请期待后续更新。目前可使用「导入」Tab 导入账号。",
                provider
            )),

        };

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(OAUTH_TIMEOUT_SECS);
        let provider_owned = provider.to_string();
        let code_verifier_opt = extra_state.clone();

        // 占位写入 SESSION_MAP
        with_sessions(|map| {
            map.insert(login_id.clone(), OAuthSession {
                provider:       provider_owned.clone(),
                callback_port:  Some(port),
                pending_result: None,
                state_token:    Some(state_token.clone()),
                code_verifier:  extra_state,
                device_code:    None,
                cancel_tx,
                expires_at,
            });
        });

        // 后台等待回调（按渠道路由不同解析逻辑）
        let login_id_bg   = login_id.clone();
        let app_handle    = app.clone();
        let state_bg      = state_token.clone();
        let redirect_uri_bg = redirect_uri.clone();
        tokio::spawn(async move {

            let oauth_result = match provider_owned.as_str() {
                "windsurf" => {
                    // Windsurf Implicit Flow：access_token 直接在回调 query 中
                    match wait_for_windsurf_callback(port, state_bg, cancel_rx).await {
                        Ok(access_token) => {
                            // 用 access_token 换 api_key（RegisterUser）
                            match windsurf_register_user(&access_token).await {
                                Ok((api_key, name, email)) => Ok(OAuthResult {
                                    token:    api_key,
                                    email,
                                    name,
                                    provider: "windsurf".to_string(),
                                    error:    None,
                                }),
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                // Google 系（antigravity / gemini）：Authorization Code Flow
                _ => {
                    let client_id_owned = match provider_owned.as_str() {
                        "gemini" => GEMINI_CLIENT_ID.to_string(),
                        _        => ANTIGRAVITY_CLIENT_ID.to_string(),
                    };
                    let client_secret_own = match provider_owned.as_str() {
                        "gemini" => GEMINI_CLIENT_SECRET.to_string(),
                        _        => ANTIGRAVITY_CLIENT_SECRET.to_string(),
                    };
                    match wait_for_callback(port, state_bg, cancel_rx).await {
                        Ok((code, redir)) => {
                            match exchange_google_code(&code, &redir, &client_id_owned, &client_secret_own).await {
                                Ok((access_token, refresh_token)) => {
                                    let user_info = fetch_google_userinfo(&access_token).await;
                                    let token = refresh_token.unwrap_or(access_token);
                                    Ok(OAuthResult {
                                        token,
                                        email:    user_info.as_ref().and_then(|u| u.email.clone()),
                                        name:     user_info.as_ref().and_then(|u| u.name.clone()),
                                        provider: provider_owned.clone(),
                                        error:    None,
                                    })
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                // Codex：PKCE + code exchange 到 auth.openai.com
                "codex" => {
                    // Codex 使用固定端口 1455
                    let cb_port = CODEX_CALLBACK_PORT;
                    match wait_for_callback(cb_port, state_bg, cancel_rx).await {
                        Ok((code, _redir)) => {
                            let verifier = code_verifier_opt.clone().unwrap_or_default();
                            let redir = format!("http://localhost:{}{}", CODEX_CALLBACK_PORT, CODEX_CALLBACK_PATH);
                            match exchange_pkce_code(CODEX_TOKEN_URL, CODEX_CLIENT_ID, &code, &redir, &verifier).await {
                                Ok(token_json) => {
                                    let access_token = token_json["access_token"].as_str().unwrap_or("").to_string();
                                    let email = decode_jwt_claim(&access_token, "email");
                                    let name = decode_jwt_claim(&access_token, "name");
                                    Ok(OAuthResult {
                                        token: access_token,
                                        email,
                                        name,
                                        provider: "codex".to_string(),
                                        error: None,
                                    })
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                // Kiro：PKCE + code exchange 到 kiro auth service
                "kiro" => {
                    match wait_for_callback(port, state_bg, cancel_rx).await {
                        Ok((code, redir)) => {
                            let verifier = code_verifier_opt.clone().unwrap_or_default();
                            match exchange_pkce_code(KIRO_TOKEN_URL, "", &code, &redir, &verifier).await {
                                Ok(token_json) => {
                                    let access_token = token_json["accessToken"]
                                        .as_str()
                                        .or_else(|| token_json["access_token"].as_str())
                                        .unwrap_or("").to_string();
                                    let email = token_json["email"].as_str().map(String::from)
                                        .or_else(|| decode_jwt_claim(&access_token, "email"));
                                    let name = token_json["name"].as_str().map(String::from)
                                        .or_else(|| decode_jwt_claim(&access_token, "name"));
                                    Ok(OAuthResult {
                                        token: access_token,
                                        email,
                                        name,
                                        provider: "kiro".to_string(),
                                        error: None,
                                    })
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                // Trae：先请求 GetLoginGuidance 获取真实登录 URL，然后等候 refresh_token 回调
                "trae" => {
                    match trae_get_login_url(&redirect_uri_bg, &state_bg).await {
                        Ok(real_auth_url) => {
                            // 用回调发现的真实 URL 替换浏览器打开的 URL（已通过 app_handle opener 打开）
                            use tauri_plugin_opener::OpenerExt;
                            let _ = app_handle.opener().open_url(&real_auth_url, None::<String>);
                            match wait_for_callback(port, state_bg.clone(), cancel_rx).await {
                                Ok((refresh_token, _redir)) => {
                                    // Trae 回调直接带 refresh_token
                                    let (email, name) = trae_get_user_info(&refresh_token, &redirect_uri_bg).await
                                        .unwrap_or((None, None));
                                    Ok(OAuthResult {
                                        token: refresh_token,
                                        email,
                                        name,
                                        provider: "trae".to_string(),
                                        error: None,
                                    })
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                // Zed：回调带 access_token（类似 Implicit Flow）
                "zed" => {
                    match wait_for_callback(port, state_bg, cancel_rx).await {
                        Ok((access_token, _)) => {
                            let email = decode_jwt_claim(&access_token, "email");
                            let name = decode_jwt_claim(&access_token, "name");
                            Ok(OAuthResult {
                                token: access_token,
                                email,
                                name,
                                provider: "zed".to_string(),
                                error: None,
                            })
                        }
                        Err(e) => Err(e),
                    }
                }
            };


            match oauth_result {
                Ok(result) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(result);
                        }
                    });
                    use tauri::Manager;
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    use tauri::Emitter;
                    let _ = app_handle.emit("oauth-callback-received", &login_id_bg);
                }
                Err(e) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(OAuthResult {
                                token: String::new(), email: None, name: None,
                                provider: String::new(), error: Some(e),
                            });
                        }
                    });
                }
            }
        });

        // 打开系统浏览器
        {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url(&auth_url, None::<String>);
        }

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code:        String::new(),
            verification_uri: auth_url,
            expires_in:       OAUTH_TIMEOUT_SECS,
            interval_seconds: 2,
        })
    }

    // ── D 类：服务端 state 轮询（Qoder / CodeBuddy）────────────────────────────

    async fn start_server_poll_flow(
        app: tauri::AppHandle,
        provider: &str,
    ) -> Result<DeviceFlowStartResponse, String> {
        let (verification_uri, poll_state, login_id) = match provider {
            "qoder" => {
                // 生成 PKCE 凭证 + nonce
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let nonce = generate_token(16);
                let login_id = generate_token(16);

                let auth_url = format!(
                    "{}?nonce={}&challenge={}&challenge_method=S256&client_id={}",
                    QODER_LOGIN_URL,
                    urlencoding::encode(&nonce),
                    urlencoding::encode(&code_challenge),
                    urlencoding::encode(QODER_CLIENT_ID)
                );

                // poll_state 编码为 JSON 字符串存 device_code 字段
                let state_json = serde_json::json!({
                    "nonce": nonce,
                    "verifier": code_verifier,
                    "challenge_method": "S256"
                }).to_string();

                (auth_url, state_json, login_id)
            }
            "codebuddy" => {
                // 先请求 auth/state
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .build()
                    .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;
                let url = format!("{}{}/auth/state?platform=ide", CODEBUDDY_API_URL, CODEBUDDY_API_PREFIX);
                let resp = client.post(&url).json(&serde_json::json!({})).send().await
                    .map_err(|e| format!("CodeBuddy auth/state 请求失败: {}", e))?;

                let body: serde_json::Value = resp.json().await
                    .map_err(|e| format!("解析 CodeBuddy auth/state 响应失败: {}", e))?;

                let data = body.get("data").cloned().unwrap_or(serde_json::Value::Null);
                let state = data.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let auth_url = data.get("authUrl").or_else(|| data.get("url"))
                    .and_then(|v| v.as_str()).filter(|s| !s.is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| format!("{}/login?state={}", CODEBUDDY_API_URL, state));

                let login_id = generate_token(16);
                let state_json = serde_json::json!({ "state": state }).to_string();

                (auth_url, state_json, login_id)
            }
            _ => return Err(format!("未知的服务端轮询渠道: {}", provider)),
        };

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(OAUTH_TIMEOUT_SECS);
        let provider_owned = provider.to_string();
        let poll_state_clone = poll_state.clone();

        with_sessions(|map| {
            map.insert(login_id.clone(), OAuthSession {
                provider:       provider_owned.clone(),
                callback_port:  None,
                pending_result: None,
                state_token:    None,
                code_verifier:  None,
                device_code:    Some(poll_state.clone()),
                cancel_tx,
                expires_at,
            });
        });

        // 后台轮询
        let login_id_bg  = login_id.clone();
        let app_bg       = app.clone();
        tokio::spawn(async move {
            let result = match provider_owned.as_str() {
                "qoder" => {
                    let state: serde_json::Value = serde_json::from_str(&poll_state_clone)
                        .unwrap_or(serde_json::Value::Null);
                    let nonce    = state["nonce"].as_str().unwrap_or("").to_string();
                    let verifier = state["verifier"].as_str().unwrap_or("").to_string();
                    let method   = state["challenge_method"].as_str().unwrap_or("S256").to_string();

                    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().ok();
                    let mut found: Option<OAuthResult> = None;
                    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS);

                    loop {
                        if tokio::time::Instant::now() > deadline { break; }
                        // 检查取消
                        if *cancel_rx.borrow() { break; }

                        if let Some(ref c) = client {
                            let url = format!("{}{}?nonce={}&verifier={}&challenge_method={}",
                                QODER_OPENAPI_URL, QODER_POLL_PATH,
                                urlencoding::encode(&nonce),
                                urlencoding::encode(&verifier),
                                urlencoding::encode(&method));
                            if let Ok(resp) = c.get(&url).send().await {
                                if resp.status().is_success() {
                                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                                        let token = body.get("token")
                                            .and_then(|v| v.as_str())
                                            .filter(|s| !s.is_empty())
                                            .map(String::from);
                                        if let Some(tk) = token {
                                            // 获取用户信息
                                            let (email, name) = if let Ok(ui_resp) = c
                                                .get(&format!("{}{}", QODER_OPENAPI_URL, QODER_USERINFO_PATH))
                                                .bearer_auth(&tk).send().await {
                                                if let Ok(ui) = ui_resp.json::<serde_json::Value>().await {
                                                    let email = ui.get("email").and_then(|v| v.as_str()).map(String::from);
                                                    let name = ui.get("name").and_then(|v| v.as_str()).map(String::from);
                                                    (email, name)
                                                } else { (None, None) }
                                            } else { (None, None) };
                                            found = Some(OAuthResult { token: tk, email, name, provider: "qoder".to_string(), error: None });
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    found.ok_or("Qoder 授权超时或已取消".to_string())
                }
                "codebuddy" => {
                    let state: serde_json::Value = serde_json::from_str(&poll_state_clone)
                        .unwrap_or(serde_json::Value::Null);
                    let cb_state = state["state"].as_str().unwrap_or("").to_string();

                    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().ok();
                    let mut found: Option<OAuthResult> = None;
                    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS);

                    loop {
                        if tokio::time::Instant::now() > deadline { break; }
                        if *cancel_rx.borrow() { break; }

                        if let Some(ref c) = client {
                            let url = format!("{}{}/auth/token?state={}", CODEBUDDY_API_URL, CODEBUDDY_API_PREFIX, cb_state);
                            if let Ok(resp) = c.get(&url).send().await {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    let code = body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
                                    if code == 0 || code == 200 {
                                        if let Some(data) = body.get("data") {
                                            let access_token = data.get("accessToken")
                                                .or_else(|| data.get("access_token"))
                                                .and_then(|v| v.as_str())
                                                .filter(|s| !s.is_empty())
                                                .map(String::from);
                                            if let Some(tk) = access_token {
                                                let email = data.get("email").and_then(|v| v.as_str()).map(String::from);
                                                let name = data.get("nickname").or_else(|| data.get("name"))
                                                    .and_then(|v| v.as_str()).map(String::from);
                                                found = Some(OAuthResult { token: tk, email, name, provider: "codebuddy".to_string(), error: None });
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    found.ok_or("CodeBuddy 授权超时或已取消".to_string())
                }
                _ => Err("未知渠道".to_string()),
            };

            match result {
                Ok(r) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(r);
                        }
                    });
                    use tauri::Manager;
                    if let Some(window) = app_bg.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    use tauri::Emitter;
                    let _ = app_bg.emit("oauth-callback-received", &login_id_bg);
                }
                Err(e) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(OAuthResult {
                                token: String::new(), email: None, name: None,
                                provider: String::new(), error: Some(e),
                            });
                        }
                    });
                }
            }
        });

        // 打开系统浏览器
        {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url(&verification_uri, None::<String>);
        }

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code:        String::new(),
            verification_uri,
            expires_in:       OAUTH_TIMEOUT_SECS,
            interval_seconds: 2,
        })
    }

    // ── B 类：GitHub Copilot Device Flow ─────────────────────────────────────


    async fn start_github_device_flow() -> Result<DeviceFlowStartResponse, String> {

        #[derive(Deserialize)]
        struct GhDeviceCodeResp {
            device_code:      String,
            user_code:        String,
            verification_uri: String,
            expires_in:       u64,
            interval:         Option<u64>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let resp = client
            .post(GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .header("User-Agent", "ai-singularity")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("scope", GITHUB_SCOPE),
            ])
            .send()
            .await
            .map_err(|e| format!("请求 GitHub 设备码失败（请检查网络）: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub 设备码请求失败: HTTP {} — {}", status, body));
        }

        let payload: GhDeviceCodeResp = resp
            .json()
            .await
            .map_err(|e| format!("解析 GitHub 设备码响应失败: {}", e))?;

        let login_id = generate_token(16);
        let interval = payload.interval.unwrap_or(5).max(5);
        let (cancel_tx, _) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(payload.expires_in);

        with_sessions(|map| {
            map.insert(login_id.clone(), OAuthSession {
                provider:       "github_copilot".to_string(),
                callback_port:  None,
                pending_result: None,
                state_token:    None,
                code_verifier:  None,
                device_code:    Some(payload.device_code.clone()),
                cancel_tx,
                expires_at,
            });
        });

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code:        payload.user_code,
            verification_uri: payload.verification_uri,
            expires_in:       payload.expires_in,
            interval_seconds: interval,
        })
    }

    // ── Cursor DeepControl ────────────────────────────────────────────────────

    fn start_cursor_flow() -> Result<DeviceFlowStartResponse, String> {
        let code_verifier  = generate_token(32);
        let code_challenge = sha256_b64(&code_verifier);
        let uuid     = uuid::Uuid::new_v4().to_string();
        let login_id = generate_token(16);

        let verification_uri = format!(
            "{}?challenge={}&uuid={}&mode=login",
            CURSOR_LOGIN_URL, code_challenge, uuid
        );

        let (cancel_tx, _) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(300);

        with_sessions(|map| {
            map.insert(login_id.clone(), OAuthSession {
                provider:       "cursor".to_string(),
                callback_port:  None,
                pending_result: None,
                state_token:    Some(uuid.clone()), // 复用 state_token 存 uuid
                code_verifier:  Some(code_verifier.clone()),
                device_code:    None,
                cancel_tx,
                expires_at,
            });
        });

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code:        String::new(),
            verification_uri,
            expires_in:       300,
            interval_seconds: 2,
        })
    }

    // ── 统一轮询接口 ──────────────────────────────────────────────────────────

    /// 前端每隔 interval_seconds 调用一次
    /// - Ok(Some(result)) → 授权成功
    /// - Ok(None)         → 继续等待
    /// - Err(msg)         → 失败/超时/取消
    pub async fn poll_oauth_login(login_id: String) -> Result<Option<OAuthResult>, String> {
        let (provider, device_code, code_verifier, uuid, expired, pending) =
            with_sessions(|map| {
                if let Some(s) = map.get(&login_id) {
                    (
                        s.provider.clone(),
                        s.device_code.clone(),
                        s.code_verifier.clone(),
                        s.state_token.clone(),
                        s.expires_at < Instant::now(),
                        s.pending_result.clone(),
                    )
                } else {
                    ("".to_string(), None, None, None, true, None)
                }
            });

        if expired {
            with_sessions(|map| { map.remove(&login_id); });
            return Err("授权码已过期，请重新发起".to_string());
        }

        // A 类：检查回调是否已写入
        if is_localhost_redirect_provider(&provider) {
            if let Some(result) = pending {
                with_sessions(|map| { map.remove(&login_id); });
                if let Some(e) = result.error {
                    return Err(e);
                }
                return Ok(Some(result));
            }
            return Ok(None);
        }

        // Cursor 轮询
        if is_cursor_provider(&provider) {
            return Self::poll_cursor(
                &login_id,
                &uuid.unwrap_or_default(),
                &code_verifier.unwrap_or_default(),
            ).await;
        }

        // B 类：GitHub Device Flow 轮询
        if is_device_flow_provider(&provider) {
            return Self::poll_github(
                &login_id,
                &device_code.unwrap_or_default(),
            ).await;
        }

        Err(format!("未知 provider: {}", provider))
    }

    // 轮询 Cursor
    async fn poll_cursor(
        login_id: &str,
        uuid: &str,
        code_verifier: &str,
    ) -> Result<Option<OAuthResult>, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CursorPollResp {
            access_token:  Option<String>,
            refresh_token: Option<String>,
            auth_id:       Option<String>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let url = format!("{}?uuid={}&verifier={}", CURSOR_POLL_URL, uuid, code_verifier);
        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Cursor 轮询请求失败: {}", e))?;

        let status = resp.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            return Ok(None);
        }

        let body: CursorPollResp = resp
            .json()
            .await
            .map_err(|_| "解析 Cursor 轮询响应失败".to_string())?;

        if let Some(token) = body.access_token.or(body.refresh_token) {
            if !token.is_empty() {
                // 从 auth_id 中提取 email（格式可能是 "user:email@example.com"）
                let email = body.auth_id.as_deref()
                    .filter(|id| id.contains('@'))
                    .map(|id| id.to_string());

                with_sessions(|map| { map.remove(login_id); });
                return Ok(Some(OAuthResult {
                    token,
                    email,
                    name: None,
                    provider: "cursor".to_string(),
                    error: None,
                }));
            }
        }
        Ok(None)
    }

    // 轮询 GitHub
    async fn poll_github(
        login_id: &str,
        device_code: &str,
    ) -> Result<Option<OAuthResult>, String> {
        #[derive(Deserialize)]
        struct GhTokenResp {
            access_token: Option<String>,
            error:        Option<String>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let resp = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .header("User-Agent", "ai-singularity")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| format!("GitHub token 请求失败: {}", e))?;

        let body: GhTokenResp = resp
            .json()
            .await
            .map_err(|e| format!("解析 GitHub token 响应失败: {}", e))?;

        if let Some(token) = body.access_token {
            if !token.is_empty() {
                // 尝试获取 GitHub 用户信息
                let email = Self::fetch_github_user_email(&token).await;
                with_sessions(|map| { map.remove(login_id); });
                return Ok(Some(OAuthResult {
                    token,
                    email,
                    name: None,
                    provider: "github_copilot".to_string(),
                    error: None,
                }));
            }
        }

        match body.error.as_deref() {
            Some("authorization_pending") | Some("slow_down") | None => Ok(None),
            Some("expired_token") => {
                with_sessions(|map| { map.remove(login_id); });
                Err("GitHub 授权码已过期，请重新发起".to_string())
            }
            Some("access_denied") => {
                with_sessions(|map| { map.remove(login_id); });
                Err("用户拒绝了授权".to_string())
            }
            Some(other) => {
                with_sessions(|map| { map.remove(login_id); });
                Err(format!("GitHub 授权失败: {}", other))
            }
        }
    }

    /// 尝试获取 GitHub 用户的主 email
    async fn fetch_github_user_email(access_token: &str) -> Option<String> {
        #[derive(Deserialize)]
        struct GhUser { login: Option<String>, email: Option<String> }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;

        let resp = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "ai-singularity")
            .send()
            .await
            .ok()?;

        let user: GhUser = resp.json().await.ok()?;
        user.email.or(user.login)
    }

    // ── 取消授权 ──────────────────────────────────────────────────────────────

    pub fn cancel_oauth_flow(login_id: Option<String>) -> Result<(), String> {
        with_sessions(|map| {
            match &login_id {
                Some(id) => {
                    if let Some(s) = map.remove(id) {
                        let _ = s.cancel_tx.send(true);
                    }
                }
                None => {
                    for (_, s) in map.drain() {
                        let _ = s.cancel_tx.send(true);
                    }
                }
            }
        });
        Ok(())
    }

    /// 兼容接口：等同于 start_oauth_flow("antigravity")
    pub async fn prepare_oauth_url(app: tauri::AppHandle) -> Result<DeviceFlowStartResponse, String> {
        Self::start_oauth_flow(app, "antigravity".to_string()).await
    }
}
