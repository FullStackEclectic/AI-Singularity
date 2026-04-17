use crate::services::session_manager::SessionManager;
use crate::services::provider_current::{CurrentAccountSnapshot, ProviderCurrentService};
use crate::services::wakeup::WakeupService;
use crate::services::websocket::{get_status as get_websocket_status, WebSocketStatus};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const REPORT_PORT_START: u16 = 32155;
const REPORT_PORT_LEN: u16 = 10;
const REPORT_AUTH_TOKEN_ENV: &str = "AIS_WEB_REPORT_TOKEN";

static REPORT_PORT: OnceLock<Option<u16>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebReportStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub local_url: Option<String>,
    pub health_url: Option<String>,
    pub status_api_url: Option<String>,
    pub snapshot_api_url: Option<String>,
    pub auth_enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebReportHealth {
    ok: bool,
    version: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebReportSnapshot {
    version: String,
    timestamp: String,
    websocket: WebSocketStatus,
    current_accounts: Vec<CurrentAccountSnapshot>,
    wakeup: WebReportWakeupSnapshot,
    sessions: WebReportSessionSnapshot,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebReportWakeupSnapshot {
    enabled: bool,
    task_count: usize,
    active_task_count: usize,
    paused_task_count: usize,
    failing_task_count: usize,
    category_counts: HashMap<String, usize>,
    last_history_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebReportSessionSnapshot {
    total: usize,
    transcript_count: usize,
    workspace_history_count: usize,
    no_transcript_count: usize,
}

pub async fn start_server(app_data_dir: PathBuf) {
    if REPORT_PORT.get().is_some() {
        return;
    }

    let mut listener = None;
    let mut bound_port = None;
    for offset in 0..REPORT_PORT_LEN {
        let port = REPORT_PORT_START + offset;
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(value) => {
                listener = Some(value);
                bound_port = Some(port);
                break;
            }
            Err(_) => continue,
        }
    }

    let _ = REPORT_PORT.set(bound_port);
    let Some(listener) = listener else {
        tracing::warn!(
            "[WebReport] 无法绑定本地端口 {}-{}",
            REPORT_PORT_START,
            REPORT_PORT_START + REPORT_PORT_LEN - 1
        );
        return;
    };

    tracing::info!(
        "[WebReport] 本地报告服务已启动: http://127.0.0.1:{}",
        bound_port.unwrap_or(REPORT_PORT_START)
    );

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let app_data_dir = app_data_dir.clone();
                tokio::spawn(async move {
                    let mut buffer = [0u8; 8192];
                    let read_len = match stream.read(&mut buffer).await {
                        Ok(len) => len,
                        Err(_) => return,
                    };
                    let request_text = String::from_utf8_lossy(&buffer[..read_len]).to_string();
                    let request = parse_request(&request_text);
                    let response = build_response(&request, &app_data_dir);
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.shutdown().await;
                });
            }
            Err(err) => {
                tracing::warn!("[WebReport] 接受连接失败: {}", err);
                break;
            }
        }
    }
}

pub fn get_port() -> Option<u16> {
    REPORT_PORT.get().copied().flatten()
}

pub fn get_status() -> WebReportStatus {
    let port = get_port();
    let local_url = port.map(|value| format!("http://127.0.0.1:{}", value));
    WebReportStatus {
        running: port.is_some(),
        port,
        health_url: local_url.as_ref().map(|base| format!("{}/healthz", base)),
        status_api_url: local_url.as_ref().map(|base| format!("{}/api/status", base)),
        snapshot_api_url: local_url
            .as_ref()
            .map(|base| format!("{}/api/snapshot", base)),
        local_url,
        auth_enabled: auth_token().is_some(),
    }
}

#[derive(Debug, Default)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

fn parse_request(raw: &str) -> HttpRequest {
    let mut lines = raw.lines();
    let first_line = lines.next().unwrap_or_default();
    let mut first_line_parts = first_line.split_whitespace();
    let method = first_line_parts.next().unwrap_or("GET").to_string();
    let path = first_line_parts.next().unwrap_or("/").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    HttpRequest {
        method,
        path,
        headers,
    }
}

fn build_response(request: &HttpRequest, app_data_dir: &Path) -> String {
    if request.method.eq_ignore_ascii_case("OPTIONS") {
        return build_empty_response(204, "No Content");
    }

    let path_only = request.path.split('?').next().unwrap_or("/");

    match path_only {
        "/" => build_html_response(build_status_page(app_data_dir)),
        "/healthz" => build_json_response(
            200,
            &WebReportHealth {
                ok: true,
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: Utc::now().to_rfc3339(),
            },
        ),
        "/api/status" => {
            if !is_authorized(request) {
                return build_json_response(
                    401,
                    &serde_json::json!({
                        "error": "unauthorized",
                        "message": "JSON 接口需要携带 Authorization: Bearer <token> 或 X-AIS-Token。",
                    }),
                );
            }
            build_json_response(200, &get_status())
        }
        "/api/snapshot" => {
            if !is_authorized(request) {
                return build_json_response(
                    401,
                    &serde_json::json!({
                        "error": "unauthorized",
                        "message": "JSON 接口需要携带 Authorization: Bearer <token> 或 X-AIS-Token。",
                    }),
                );
            }
            build_json_response(200, &build_snapshot(app_data_dir))
        }
        _ => build_json_response(
            404,
            &serde_json::json!({
                "error": "not_found",
                "message": "未找到对应的 Web report 路径。",
            }),
        ),
    }
}

fn build_snapshot(app_data_dir: &Path) -> WebReportSnapshot {
    let websocket = get_websocket_status();
    let db_path = app_data_dir.join("data.db");
    let current_accounts = crate::db::Database::new(&db_path)
        .ok()
        .and_then(|db| ProviderCurrentService::list_current_account_snapshots(&db).ok())
        .unwrap_or_default();
    let wakeup_state = WakeupService::load_state(app_data_dir).unwrap_or_default();
    let wakeup_history = WakeupService::load_history(app_data_dir).unwrap_or_default();
    let sessions = SessionManager::list_sessions().unwrap_or_default();

    let mut category_counts = HashMap::<String, usize>::new();
    for task in &wakeup_state.tasks {
        if let Some(category) = task.last_category.clone().filter(|value| !value.trim().is_empty()) {
            *category_counts.entry(category).or_insert(0) += 1;
        }
    }

    let transcript_count = sessions.iter().filter(|item| item.messages_count > 0).count();
    let workspace_history_count = sessions
        .iter()
        .filter(|item| item.source_kind.as_deref() == Some("workspace_history"))
        .count();

    WebReportSnapshot {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now().to_rfc3339(),
        websocket,
        current_accounts,
        wakeup: WebReportWakeupSnapshot {
            enabled: wakeup_state.enabled,
            task_count: wakeup_state.tasks.len(),
            active_task_count: wakeup_state.tasks.iter().filter(|item| item.enabled).count(),
            paused_task_count: wakeup_state.tasks.iter().filter(|item| !item.enabled).count(),
            failing_task_count: wakeup_state
                .tasks
                .iter()
                .filter(|item| item.consecutive_failures > 0)
                .count(),
            category_counts,
            last_history_at: wakeup_history.first().map(|item| item.created_at.clone()),
        },
        sessions: WebReportSessionSnapshot {
            total: sessions.len(),
            transcript_count,
            workspace_history_count,
            no_transcript_count: sessions.len().saturating_sub(transcript_count),
        },
    }
}

fn auth_token() -> Option<String> {
    std::env::var(REPORT_AUTH_TOKEN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_authorized(request: &HttpRequest) -> bool {
    let Some(expected) = auth_token() else {
        return true;
    };

    if let Some(token) = request.headers.get("x-ais-token") {
        return token == &expected;
    }

    if let Some(header) = request.headers.get("authorization") {
        if let Some(token) = header.strip_prefix("Bearer ") {
            return token.trim() == expected;
        }
    }

    false
}

fn build_status_page(app_data_dir: &Path) -> String {
    let ws = get_websocket_status();
    let report = get_status();
    let current_accounts = crate::db::Database::new(&app_data_dir.join("data.db"))
        .ok()
        .and_then(|db| ProviderCurrentService::list_current_account_snapshots(&db).ok())
        .unwrap_or_default();
    let json_hint = if report.auth_enabled {
        "JSON 接口已启用 token 校验，请在客户端请求时携带 Authorization: Bearer <token> 或 X-AIS-Token。"
    } else {
        "JSON 接口当前未启用额外认证，适合本机或受信环境内的外部客户端直接探活。"
    };
    let current_accounts_html = if current_accounts.is_empty() {
        "<div class=\"meta\">当前没有可展示的账号快照。</div>".to_string()
    } else {
        format!(
            "<div class=\"links\">{}</div>",
            current_accounts
                .into_iter()
                .map(|item| {
                    format!(
                        "<div><code>{}</code> {}</div>",
                        item.platform,
                        item.label
                            .or(item.email)
                            .unwrap_or_else(|| "未解析到当前账号".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        )
    };

    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>AI Singularity Local Report</title>
  <style>
    body {{ font-family: Segoe UI, Arial, sans-serif; background:#0b1220; color:#e5eefc; padding:32px; }}
    .card {{ max-width:820px; margin:0 auto; padding:24px; border-radius:20px; background:#101a2c; border:1px solid rgba(255,255,255,0.08); }}
    .meta {{ color:#93a8c8; margin-top:8px; line-height:1.6; }}
    .grid {{ display:grid; grid-template-columns:repeat(4,1fr); gap:16px; margin-top:20px; }}
    .item {{ padding:16px; border-radius:16px; background:#0d1628; border:1px solid rgba(255,255,255,0.08); }}
    .value {{ font-size:26px; font-weight:700; margin-top:8px; }}
    .label {{ color:#8ea1bf; font-size:13px; }}
    .links {{ margin-top:18px; display:flex; flex-direction:column; gap:8px; }}
    a {{ color:#8bd3ff; text-decoration:none; }}
    code {{ color:#c5d7f2; }}
    @media (max-width: 760px) {{ .grid {{ grid-template-columns:repeat(2,1fr); }} body {{ padding:16px; }} }}
    @media (max-width: 520px) {{ .grid {{ grid-template-columns:1fr; }} }}
  </style>
</head>
<body>
  <div class="card">
    <h1>AI Singularity Local Report</h1>
    <div class="meta">本地状态页，可用于桌面外客户端确认广播服务、会话快照与 Wakeup 调度是否在线。</div>
    <div class="grid">
      <div class="item">
        <div class="label">版本</div>
        <div class="value">{}</div>
      </div>
      <div class="item">
        <div class="label">WebSocket</div>
        <div class="value">{}</div>
      </div>
      <div class="item">
        <div class="label">连接客户端</div>
        <div class="value">{}</div>
      </div>
      <div class="item">
        <div class="label">JSON 认证</div>
        <div class="value">{}</div>
      </div>
    </div>
    <div class="links">
      <div><a href="/healthz">/healthz</a> <code>无需认证</code></div>
      <div><code>/api/status</code> JSON 状态接口</div>
      <div><code>/api/snapshot</code> JSON 快照接口</div>
    </div>
    <div class="meta">当前账号快照</div>
    {}
    <div class="meta">WebSocket 端口：{}</div>
    <div class="meta">{}</div>
  </div>
</body>
</html>"#,
        env!("CARGO_PKG_VERSION"),
        if ws.running { "在线" } else { "离线" },
        ws.client_count,
        if report.auth_enabled { "已开启" } else { "未开启" },
        current_accounts_html,
        ws.port
            .map(|value| value.to_string())
            .unwrap_or_else(|| "—".to_string()),
        json_hint,
    )
}

fn build_empty_response(status_code: u16, status_text: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, X-AIS-Token\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nConnection: close\r\n\r\n",
        status_code, status_text
    )
}

fn build_html_response(body: String) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, X-AIS-Token\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn build_json_response<T: Serialize>(status_code: u16, payload: &T) -> String {
    let status_text = match status_code {
        200 => "OK",
        204 => "No Content",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "OK",
    };
    let body = serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string());
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, X-AIS-Token\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nConnection: close\r\n\r\n{}",
        status_code,
        status_text,
        body.len(),
        body
    )
}
