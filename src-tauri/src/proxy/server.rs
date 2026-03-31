/// 本地 OpenAI 兼容 HTTP 代理服务器（基于 axum）
/// 接受标准 OpenAI 格式，自动路由到最优账号/平台，并处理协议转换
use crate::db::Database;
use crate::models::Platform;
use crate::proxy::converter::{
    openai_to_anthropic, openai_to_gemini, anthropic_to_openai_response, gemini_to_openai_response,
    OpenAIRequest,
};
use crate::proxy::router::Router;
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router as AxumRouter,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct ProxyState {
    pub router: Arc<Router>,
    pub http_client: reqwest::Client,
}

pub struct ProxyServer {
    port: u16,
    state: ProxyState,
}

impl ProxyServer {
    pub fn new(db: Arc<Database>, port: u16) -> Self {
        Self {
            port,
            state: ProxyState {
                router: Arc::new(Router::new(db)),
                http_client: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .unwrap(),
            },
        }
    }

    pub async fn start(self) -> anyhow::Result<()> {
        let app = AxumRouter::new()
            .route("/v1/chat/completions", post(handle_chat_completions))
            .route("/v1/models", axum::routing::get(handle_list_models))
            .with_state(self.state);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("🚀 AI Singularity 代理已启动：http://{}", addr);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

/// 处理 /v1/chat/completions 请求
async fn handle_chat_completions(
    State(state): State<ProxyState>,
    Json(body): Json<OpenAIRequest>,
) -> impl IntoResponse {
    // 1. 从路由引擎选最优 Key
    let Some(target) = state.router.pick_best_key(None) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": {"message": "没有可用的 API Key，请先在 AI Singularity 中添加", "type": "no_keys"}})),
        ).into_response();
    };

    // 2. 根据平台转换请求并转发
    let result = match &target.platform {
        Platform::Anthropic => forward_to_anthropic(&state.http_client, &target.secret, &body).await,
        Platform::Gemini => forward_to_gemini(&state.http_client, &target.secret, &body).await,
        _ => {
            // OpenAI 兼容接口（OpenAI / DeepSeek / Aliyun / Moonshot 等）
            let base = target.base_url.as_deref()
                .unwrap_or(platform_base_url(&target.platform));
            forward_to_openai_compatible(&state.http_client, &target.secret, base, &body).await
        }
    };

    match result {
        Ok(resp_body) => (StatusCode::OK, Json(resp_body)).into_response(),
        Err(e) => {
            // 标记 Key 失败
            if e.to_string().contains("401") {
                state.router.mark_key_status(&target.key_id, "invalid");
            } else if e.to_string().contains("429") {
                state.router.mark_key_status(&target.key_id, "rate_limit");
            }
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": {"message": e.to_string(), "type": "proxy_error"}})),
            ).into_response()
        }
    }
}

/// 转发到 OpenAI 兼容接口
async fn forward_to_openai_compatible(
    client: &reqwest::Client,
    secret: &str,
    base_url: &str,
    body: &OpenAIRequest,
) -> anyhow::Result<Value> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", secret))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    let json: Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("{} - {}", status.as_u16(), json);
    }

    Ok(json)
}

/// 转发到 Anthropic API（协议转换）
async fn forward_to_anthropic(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
) -> anyhow::Result<Value> {
    let anthropic_body = openai_to_anthropic(body);

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", secret)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&anthropic_body)
        .send()
        .await?;

    let status = resp.status();
    let json: Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("{} - {}", status.as_u16(), json);
    }

    Ok(anthropic_to_openai_response(&json))
}

/// 转发到 Gemini API（协议转换）
async fn forward_to_gemini(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
) -> anyhow::Result<Value> {
    let (model, gemini_body) = openai_to_gemini(body);
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, secret
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&gemini_body)
        .send()
        .await?;

    let status = resp.status();
    let json: Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("{} - {}", status.as_u16(), json);
    }

    Ok(gemini_to_openai_response(&json, &model))
}

/// 处理 /v1/models 请求（返回可用模型列表）
async fn handle_list_models(State(state): State<ProxyState>) -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [
            {"id": "gpt-4o", "object": "model"},
            {"id": "gpt-4o-mini", "object": "model"},
            {"id": "claude-3-5-sonnet-20241022", "object": "model"},
            {"id": "claude-3-haiku-20240307", "object": "model"},
            {"id": "gemini-2.0-flash", "object": "model"},
            {"id": "deepseek-chat", "object": "model"},
        ]
    }))
}

/// 平台默认 base URL
fn platform_base_url(platform: &Platform) -> &'static str {
    match platform {
        Platform::OpenAI   => "https://api.openai.com",
        Platform::DeepSeek => "https://api.deepseek.com",
        Platform::Aliyun   => "https://dashscope.aliyuncs.com/compatible-mode",
        Platform::Moonshot => "https://api.moonshot.cn",
        Platform::Zhipu    => "https://open.bigmodel.cn/api/paas",
        Platform::Bytedance => "https://ark.cn-beijing.volces.com/api",
        _                  => "https://api.openai.com",
    }
}
