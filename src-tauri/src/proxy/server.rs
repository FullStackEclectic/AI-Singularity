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

/// 获取目标模型对应的平台 ID
fn get_platform_for_model(model: &str) -> Option<&'static str> {
    let lower = model.to_lowercase();
    if lower.contains("claude") {
        Some("anthropic")
    } else if lower.contains("gemini") {
        Some("gemini")
    } else if lower.contains("gpt") || lower.contains("o1") || lower.contains("o3") {
        Some("openai")
    } else if lower.contains("deepseek") {
        Some("deep_seek")
    } else if lower.contains("qwen") || lower.contains("bailian") {
        Some("aliyun")
    } else if lower.contains("moonshot") {
        Some("moonshot")
    } else if lower.contains("glm") {
        Some("zhipu")
    } else if lower.contains("doubao") {
        Some("bytedance")
    } else {
        None
    }
}

/// 获取智能回退模型（当目标提供商超限或无密钥时）
fn get_fallback_model(model: &str) -> Option<&'static str> {
    let lower = model.to_lowercase();
    if lower.contains("claude-3-5") || lower.contains("claude-3-7") {
        Some("gemini-1.5-flash")
    } else if lower.contains("claude") {
        Some("gemini-1.5-pro")
    } else if lower.contains("gpt-4") || lower.contains("gpt-4o") || lower.contains("o1") || lower.contains("o3") {
        Some("gemini-1.5-pro")
    } else if lower.contains("gpt-3.5") || lower.contains("gpt-4o-mini") {
        Some("gemini-1.5-flash")
    } else if lower.contains("deepseek") {
        Some("gemini-1.5-flash")
    } else {
        None
    }
}

/// 处理 /v1/chat/completions 请求
async fn handle_chat_completions(
    State(state): State<ProxyState>,
    Json(mut body): Json<OpenAIRequest>,
) -> impl IntoResponse {
    // 跨模态特征指令截杀 (Image Intercept)
    if let Some(img_prompt) = extract_image_prompt(&body.messages) {
        tracing::info!("🎨 检测到绘图指令跨模态截留: {}", img_prompt);
        if let Some(target) = state.router.pick_best_key(Some("gemini")) {
            match forward_to_gemini_imagen3(&state.http_client, &target.secret, &img_prompt).await {
                Ok(resp) => return (StatusCode::OK, Json(resp)).into_response(),
                Err(e) => tracing::warn!("Gemini Imagen 3 失败: {}, 降级为外部公开代理绘图", e),
            }
        }
        // 降级 / 兜底机制
        return (StatusCode::OK, Json(construct_pollinations_response(&img_prompt))).into_response();
    }

    let mut current_model = body.model.clone();
    let mut attempts = 0;

    loop {
        attempts += 1;
        let platform_str = get_platform_for_model(&current_model);

        // 1. 从路由引擎选最优 Key
        let Some(target) = state.router.pick_best_key(platform_str) else {
            if attempts == 1 {
                if let Some(fallback) = get_fallback_model(&current_model) {
                    tracing::warn!("⚠️ 模型 {} 无可用 Provider，正在智能回退至备选模型 {}", current_model, fallback);
                    current_model = fallback.to_string();
                    body.model = current_model.clone();
                    continue;
                }
            }
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": {"message": format!("没有可用于模型 {} 的 API Key，请先添加对应平台 ({:?}) 密钥配置或备选回退方案", current_model, platform_str), "type": "no_keys"}})),
            ).into_response();
        };

        // 2. 根据平台转换请求并转发
        let result = match &target.platform {
            Platform::Anthropic => forward_to_anthropic(&state.http_client, &target.secret, &body).await,
            Platform::Gemini => forward_to_gemini(&state.http_client, &target.secret, &body).await,
            _ => {
                // OpenAI 兼容接口
                let base = target.base_url.as_deref()
                    .unwrap_or(platform_base_url(&target.platform));
                forward_to_openai_compatible(&state.http_client, &target.secret, base, &body).await
            }
        };

        match result {
            Ok(resp_body) => {
                if attempts > 1 {
                    tracing::info!("✅ 动态灾备成功：已通过回退模型 {} 完成调用", current_model);
                }
                return (StatusCode::OK, Json(resp_body)).into_response();
            }
            Err(e) => {
                let err_str = e.to_string();
                
                // 标记 Key 失败状态以便路由更新
                if err_str.contains("401") || err_str.contains("403") {
                    state.router.mark_key_status(&target.key_id, "invalid");
                } else if err_str.contains("429") {
                    state.router.mark_key_status(&target.key_id, "rate_limit");
                }

                // 如果是第一次尝试并且遭遇过载/超时等错误，触发灾备切流
                if attempts == 1 {
                    let should_fallback = err_str.contains("429") 
                        || err_str.contains("50") // 500, 502, 503, 504
                        || err_str.contains("timeout");

                    if should_fallback {
                        if let Some(fallback) = get_fallback_model(&current_model) {
                            tracing::warn!("⚠️ 请求模型 {} 发生错误 ({})，触发灾备中心，正静默切流至备选模型 {}", current_model, err_str.lines().next().unwrap_or(&err_str), fallback);
                            current_model = fallback.to_string();
                            body.model = current_model.clone();
                            continue;
                        }
                    }
                }

                // 无回退模型或已经是失败回退，直接抛出
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"error": {"message": format!("代理上游错误: {}", err_str), "type": "proxy_error"}})),
                ).into_response();
            }
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

// ==========================================
// Image Intercept Handlers
// ==========================================

fn extract_image_prompt(messages: &[crate::proxy::converter::OpenAIMessage]) -> Option<String> {
    if let Some(msg) = messages.last() {
        if msg.role == "user" {
            let content = msg.content.trim().to_lowercase();
            if content.starts_with("draw a picture of") {
                return Some(msg.content[17..].trim().to_string());
            } else if content.starts_with("画一张") {
                return Some(msg.content[9..].trim().to_string()); // "画一张" is 9 bytes in UTF-8
            } else if content.starts_with("生成图片") {
                return Some(msg.content[12..].trim().to_string()); // "生成图片" is 12 bytes
            }
        }
    }
    None
}

async fn forward_to_gemini_imagen3(
    client: &reqwest::Client,
    secret: &str,
    prompt: &str,
) -> anyhow::Result<Value> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-3.0-generate-001:predict?key={}",
        secret
    );

    let body = json!({
        "instances": [
            { "prompt": prompt }
        ],
        "parameters": {
            "sampleCount": 1,
            "aspectRatio": "1:1"
        }
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let json: Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("{} - {}", status.as_u16(), json);
    }

    let b64 = json["predictions"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|p| p["bytesBase64Encoded"].as_str())
        .unwrap_or("");

    if b64.is_empty() {
        anyhow::bail!("Imagen API returned empty base64 data");
    }

    Ok(json!({
        "id": format!("imagen-{}", chrono::Utc::now().timestamp()),
        "object": "chat.completion",
        "model": "imagen-3.0-generate-001",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Here is your generated image:\n\n![Generated Image](data:image/jpeg;base64,{})", b64),
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        }
    }))
}

fn construct_pollinations_response(prompt: &str) -> Value {
    // using purely free text-to-image without API
    let encoded: String = url::form_urlencoded::byte_serialize(prompt.as_bytes()).collect();
    let img_url = format!("https://image.pollinations.ai/prompt/{}?width=1024&height=1024&nologo=true", encoded);
    json!({
        "id": format!("pollinations-{}", chrono::Utc::now().timestamp()),
        "object": "chat.completion",
        "model": "pollinations-free",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Here is your generated image via free proxy:\n\n![Generated Image]({})", img_url),
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        }
    })
}
