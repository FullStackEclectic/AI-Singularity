/// 本地 OpenAI 兼容 HTTP 代理服务器（基于 axum）
/// 接受标准 OpenAI 格式，自动路由到最优账号/平台，并处理协议转换
use crate::db::Database;
use crate::models::Platform;
use crate::proxy::converter::OpenAIRequest;
use crate::proxy::router::Router;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::post,
    Json, Router as AxumRouter,
};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

mod helpers;
mod orchestration;
mod preprocess;
mod request;
mod transport;

use self::orchestration::forward_chat_completion;
use self::preprocess::{apply_model_mappings, compress_context_if_needed};
use self::request::{handle_image_intercept, resolve_request_context};

#[derive(Clone)]
pub struct ProxyState {
    pub router: Arc<Router>,
    pub db: Arc<Database>,
    pub http_client: reqwest::Client,
}

pub struct ProxyServer {
    port: u16,
    state: ProxyState,
}

impl ProxyServer {
    pub fn new(db: Arc<Database>, port: u16) -> Self {
        // 同步拉取安全防火墙的黑白名单规则
        if let Err(e) = crate::proxy::security::SecurityShield::sync_rules(&db) {
            tracing::warn!("⚠️ 启动时同步黑白名单规则失败: {}", e);
        }

        Self {
            port,
            state: ProxyState {
                router: Arc::new(Router::new(db.clone())),
                db,
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

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;
        Ok(())
    }
}

/// 统一转发网关模型定义
pub(super) struct ForwardTarget {
    secret: String,
    base_url: Option<String>,
    platform: Platform,
    key_id: String,
    device_profile: Option<crate::models::DeviceProfile>,
}

/// 处理 /v1/chat/completions 请求
async fn handle_chat_completions(
    State(state): State<ProxyState>,
    client_ip: axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(mut body): Json<OpenAIRequest>,
) -> impl IntoResponse {
    let request_ctx = match resolve_request_context(&state, &client_ip, &headers).await {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    if let Some(resp) = handle_image_intercept(&state, &body, &request_ctx.target_scope).await {
        return resp;
    }

    let is_stream = body.stream.unwrap_or(false);
    let mut current_model = body.model.clone();

    current_model = apply_model_mappings(&state.db, &current_model);
    compress_context_if_needed(&mut body);

    forward_chat_completion(&state, &request_ctx, &mut body, current_model, is_stream).await
}

/// 处理 /v1/models 请求（返回可用模型列表）
async fn handle_list_models(State(_state): State<ProxyState>) -> impl IntoResponse {
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
