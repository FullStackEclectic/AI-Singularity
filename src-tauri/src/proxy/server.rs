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
    response::{IntoResponse, sse::{Event, Sse}},
    routing::post,
    Json, Router as AxumRouter,
};
use futures::stream::{StreamExt, Stream};
use reqwest_eventsource::{EventSource, Event as ReqwestEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use crate::services::account_pool::AccountPoolManager;

#[derive(Clone)]
pub struct ProxyState {
    pub router: Arc<Router>,
    pub db: Arc<Database>,
    pub http_client: reqwest::Client,
    pub account_pool: Arc<AccountPoolManager>,
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
                router: Arc::new(Router::new(db.clone())),
                account_pool: Arc::new(AccountPoolManager::new(db.clone())),
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

        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
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

/// 统一转发网关模型定义
pub struct ForwardTarget {
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
    // 1. SaaS 云端管控：验证 UserToken 及其宵禁/IP 等策略
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let mut current_user_token = None;
    
    // 如果存在 sk-ag-xxx 开始的专属分发 Token，则执行核查
    if let Some(auth) = auth_header {
        if auth.starts_with("Bearer sk-ag-") {
            let token_str = &auth[7..];
            let token_service = crate::services::user_token::UserTokenService::new(&state.db);
            
            match token_service.get_token_by_str(token_str) {
                Ok(Some(ut)) => {
                    if !ut.enabled {
                        return (StatusCode::FORBIDDEN, Json(json!({"error": {"message": "Token 已被主脑冻结"}}))).into_response();
                    }
                    let now = chrono::Utc::now().timestamp();
                    if ut.expires_type == "absolute" {
                        if let Some(exp) = ut.expires_at {
                            if now > exp {
                                return (StatusCode::FORBIDDEN, Json(json!({"error": {"message": "Token 绝对有效期已过"}}))).into_response();
                            }
                        }
                    } else if ut.expires_type == "relative" {
                        if let Some(exp) = ut.expires_at {
                            if now > ut.created_at + exp {
                                return (StatusCode::FORBIDDEN, Json(json!({"error": {"message": "Token 相对配额期已过"}}))).into_response();
                            }
                        }
                    }
                    
                    // 宵禁检查
                    if let (Some(cs), Some(ce)) = (&ut.curfew_start, &ut.curfew_end) {
                        let cur_time = chrono::Utc::now().format("%H:%M").to_string();
                        if cs < ce {
                            if cur_time < *cs || cur_time > *ce {
                                return (StatusCode::FORBIDDEN, Json(json!({"error": {"message": format!("当前处于系统宵禁时段外 (允许: {}-{})", cs, ce)}}))).into_response();
                            }
                        } else {
                            // 跨夜逻辑 (22:00 - 08:00)
                            if cur_time < *cs && cur_time > *ce {
                                return (StatusCode::FORBIDDEN, Json(json!({"error": {"message": format!("当前处于跨夜系统宵禁时段外 (允许: {}-{})", cs, ce)}}))).into_response();
                            }
                        }
                    }
                    
                    current_user_token = Some(ut.id);
                }
                _ => return (StatusCode::UNAUTHORIZED, Json(json!({"error": {"message": "无效的 AI Singularity 下发 Token"}}))).into_response(),
            }
        }
    }

    // 限制外网无鉴权访问 (粗略防御)
    if current_user_token.is_none() && !client_ip.0.ip().is_loopback() {
         return (StatusCode::UNAUTHORIZED, Json(json!({"error": {"message": "外部 IP 必须携带合法的网关分发 Token (sk-ag-...)"}}))).into_response();
    }

    let client_app = headers.get("x-client-app")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("user-agent").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();
    
    let target_scope = if let Some(auth) = auth_header {
        if auth.starts_with("Bearer sk-ag-") {
            let token_str = &auth[7..];
            let token_service = crate::services::user_token::UserTokenService::new(&state.db);
            if let Ok(Some(ut)) = token_service.get_token_by_str(token_str) {
                ut.parse_scope()
            } else {
                crate::models::TokenScope {
                    scope: "global".to_string(),
                    desc: None,
                    channels: vec![],
                    tags: vec![],
                    single_account: None,
                }
            }
        } else {
            crate::models::TokenScope {
                scope: "global".to_string(),
                desc: None,
                channels: vec![],
                tags: vec![],
                single_account: None,
            }
        }
    } else {
        crate::models::TokenScope {
            scope: "global".to_string(),
            desc: None,
            channels: vec![],
            tags: vec![],
            single_account: None,
        }
    };

    // 跨模态特征指令截杀 (Image Intercept)
    if let Some(img_prompt) = extract_image_prompt(&body.messages) {
        tracing::info!("🎨 检测到绘图指令跨模态截留: {}", img_prompt);
        if let Some(target) = state.router.pick_best_key(Some("gemini"), &target_scope) {
            match forward_to_gemini_imagen3(&state.http_client, &target.secret, &img_prompt).await {
                Ok(resp) => return (StatusCode::OK, Json(resp)).into_response(),
                Err(e) => tracing::warn!("Gemini Imagen 3 失败: {}, 降级为外部公开代理绘图", e),
            }
        }
        // 降级 / 兜底机制
        return (StatusCode::OK, Json(construct_pollinations_response(&img_prompt))).into_response();
    }

    let is_stream = body.stream.unwrap_or(false);
    let mut current_model = body.model.clone();
    
    // --- 动态模型映射 (Model Mappings) 拦截 ---
    if let Ok(mappings) = crate::services::model_mapping::ModelMappingService::new(&state.db).get_all() {
        for m in mappings {
            if m.is_active && current_model.eq_ignore_ascii_case(&m.source_model) {
                tracing::info!("🔄 模型重写触发: {} => {}", current_model, m.target_model);
                current_model = m.target_model;
                break;
            }
        }
    }

    let mut attempts = 0;    loop {
        attempts += 1;
        let platform_str = get_platform_for_model(&current_model);

        // 使用重构后的 pick_best_key，传入平台限制以及分发规则作用域 (Scope)
        let target_res = state.router.pick_best_key(platform_str, &target_scope);
        
        let target = if let Some(t) = target_res {
            ForwardTarget {
                secret: t.secret,
                base_url: t.base_url,
                platform: t.platform,
                key_id: t.key_id,
                // 由于目前的重构将 device_profile 提取留在了后续的独立服务逻辑，暂时填充为 none，后续深度劫持池里再补充
                device_profile: None, 
            }
        } else {
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
                Json(json!({"error": {"message": format!("降维打击网关失败：无任何可用令牌满足当前 Scope [{}] 的分发规则，且无法进行灾备 {}", target_scope.scope, current_model), "type": "no_keys_in_scope"}})),
            ).into_response();
        };

        // 2. 根据平台转换请求并转发，携带指纹信息
        // 流式审计上下文
        let audit_ctx = AuditContext {
            db: state.db.clone(),
            key_id: target.key_id.clone(),
            platform: format!("{:?}", target.platform),
            model: current_model.clone(),
            client_app: client_app.clone(),
        };

        let response_or_stream = if is_stream {
            match &target.platform {
                Platform::Auth0IDE | Platform::Anthropic => {
                    handle_anthropic_stream(&state.http_client, &target.secret, &body, target.device_profile.as_ref(), audit_ctx).await
                }
                Platform::Gemini => {
                    handle_openai_compatible_stream(&state.http_client, &target.secret, platform_base_url(&target.platform), &body, target.device_profile.as_ref(), audit_ctx).await
                }
                _ => {
                    let base = target.base_url.as_deref().unwrap_or(platform_base_url(&target.platform));
                    handle_openai_compatible_stream(&state.http_client, &target.secret, base, &body, target.device_profile.as_ref(), audit_ctx).await
                }
            }
        } else {
            let result = match &target.platform {
                Platform::Auth0IDE => forward_to_ide_bypass(&state.http_client, &target.secret, &body, target.device_profile.as_ref()).await,
                Platform::Anthropic => forward_to_anthropic(&state.http_client, &target.secret, &body, target.device_profile.as_ref()).await,
                Platform::Gemini => forward_to_gemini(&state.http_client, &target.secret, &body).await,
                _ => {
                    let base = target.base_url.as_deref()
                        .unwrap_or(platform_base_url(&target.platform));
                    forward_to_openai_compatible(&state.http_client, &target.secret, base, &body, target.device_profile.as_ref()).await
                }
            };
            result.map(|json_val| {
                if let Some(usage) = json_val.get("usage") {
                    let prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let completion_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total_tokens = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    
                    let cost = crate::services::pricing::PricingEngine::calculate_cost(&current_model, prompt_tokens, completion_tokens);
                    let id = uuid::Uuid::new_v4().to_string();
                    let _ = state.db.execute(
                        "INSERT INTO token_usage_records (id, key_id, platform, model_name, client_app, prompt_tokens, completion_tokens, total_tokens, total_cost_usd, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
                        &[
                            &id,
                            &target.key_id,
                            &format!("{:?}", target.platform),
                            &current_model,
                            &client_app,
                            &prompt_tokens,
                            &completion_tokens,
                            &total_tokens,
                            &cost,
                        ]
                    );
                }
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&json_val).unwrap()))
                    .unwrap()
            })
        };

        match response_or_stream {
            Ok(axum_resp) => {
                if attempts > 1 {
                    tracing::info!("✅ 动态灾备成功：已通过回退模型 {} 完成调用", current_model);
                }
                // 注意：由于转为流式处理等变更，Token 日志审计逻辑这里暂且略过，流式日志可在转接层做异步上报。
                // 若要保留，需要在这里单独判断并劫持 Stream/Body，目前先保持网关高优通行。
                
                return axum_resp.into_response();
            }
            Err(e) => {
                let err_str = e.to_string();
                
                // 标记 Key 失败状态以便路由更新
                if target.platform == Platform::Auth0IDE || target.device_profile.is_some() {
                    // 对于 IDE account，目前可以认为是走的 ide account 逻辑。严格来说我们在 router 能区分，不过目前 server 只根据 device proxy。
                    // 但是因为现在的 router 其实隐藏了来源，最好在 mark_key_status 里面根据 ID 前缀判断，或者再 router 改一下抛出。
                    // 我们前面已经改了 mark_key_status，需要传 boolis_ide_account。对于 Auth0IDE，或者在我们的设计中如果 key_id 通常是 ide_accounts 的，可以判断。
                    // 最好如果 device_profile 存在或者 platform 是 IDE 的传入 true。
                    // 这里为了简化，原代码其实也是拆分的，现在我们统一传：
                    if err_str.contains("401") || err_str.contains("403") {
                        state.router.mark_key_status(&target.key_id, true, "forbidden");
                    } else if err_str.contains("429") {
                        state.router.mark_key_status(&target.key_id, true, "rate_limited");
                    }
                } else {
                    if err_str.contains("401") || err_str.contains("403") {
                        state.router.mark_key_status(&target.key_id, false, "invalid");
                    } else if err_str.contains("429") {
                        state.router.mark_key_status(&target.key_id, false, "rate_limit");
                    }
                }

                let should_fallback = err_str.contains("429") 
                    || err_str.contains("50") // 500, 502, 503, 504
                    || err_str.contains("timeout");

                if should_fallback {
                    if attempts < 3 {
                        tracing::warn!("⚠️ 节点 [{}] (平台: {:?}) 发生熔断 ({})，触发毫秒级切流 (已尝试: {})", target.key_id, target.platform, err_str.lines().next().unwrap_or(&err_str), attempts);
                        continue;
                    }
                    if let Some(fallback) = get_fallback_model(&current_model) {
                        tracing::warn!("⚠️ 连续熔断！模型 {} 不可用，降灾备份至 {}", current_model, fallback);
                        current_model = fallback.to_string();
                        body.model = current_model.clone();
                        attempts = 0; // 重置尝试次数，下一次循环用备选模型起手
                        continue;
                    }
                }

                // 无回退模型或已经是失败回退，直接抛出
                return axum::response::Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&json!({"error": {"message": format!("代理上游错误: {}", err_str), "type": "proxy_error"}})).unwrap()))
                    .unwrap()
                    .into_response();
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
    device_profile: Option<&crate::models::DeviceProfile>,
) -> anyhow::Result<Value> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    
    let mut request = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", secret))
        .header("Content-Type", "application/json");

    // 🏆 **核武护城河：机器指纹拦截欺骗**
    if let Some(dp) = device_profile {
        tracing::info!("🛡️ 启用指纹降维伪装 [IDE]: 正在向游离服务点重写物理设备信息 - MachineID: {}", dp.machine_id);
        request = request
            .header("x-machine-id", &dp.machine_id)
            .header("x-mac-machine-id", &dp.mac_machine_id)
            .header("x-dev-device-id", &dp.dev_device_id)
            .header("x-sqm-id", &dp.sqm_id);
    }

    let resp = request.json(body).send().await?;

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
    device_profile: Option<&crate::models::DeviceProfile>,
) -> anyhow::Result<Value> {
    let anthropic_body = openai_to_anthropic(body);

    let mut request = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", secret)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json");

    if let Some(dp) = device_profile {
        tracing::info!("🛡️ 启用指纹降维伪装 [Anthropic]: 正在向服务器重写物理指纹 - MachineID: {}", dp.machine_id);
        request = request
            .header("x-machine-id", &dp.machine_id)
            .header("x-mac-machine-id", &dp.mac_machine_id);
    }

    let resp = request.json(&anthropic_body).send().await?;

    let status = resp.status();
    let json: Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("{} - {}", status.as_u16(), json);
    }

    Ok(anthropic_to_openai_response(&json))
}

/// 审计上下文，供流式函数使用
struct AuditContext {
    db: Arc<Database>,
    key_id: String,
    platform: String,
    model: String,
    client_app: String,
}

impl AuditContext {
    fn write_usage(&self, prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) {
        let cost = crate::services::pricing::PricingEngine::calculate_cost(&self.model, prompt_tokens, completion_tokens);
        let id = uuid::Uuid::new_v4().to_string();
        let _ = self.db.execute(
            "INSERT INTO token_usage_records (id, key_id, platform, model_name, client_app, prompt_tokens, completion_tokens, total_tokens, total_cost_usd, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
            &[
                &id,
                &self.key_id,
                &self.platform,
                &self.model,
                &self.client_app,
                &prompt_tokens,
                &completion_tokens,
                &total_tokens,
                &cost,
            ]
        );
    }
}

async fn handle_anthropic_stream(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
    audit: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    use crate::proxy::mappers::{ProtocolMapper, anthropic::AnthropicMapper};
    
    let anthropic_body = openai_to_anthropic(body);
    let mut request = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", secret)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json");

    if let Some(dp) = device_profile {
        request = request
            .header("x-machine-id", &dp.machine_id)
            .header("x-mac-machine-id", &dp.mac_machine_id);
    }

    let req = request.json(&anthropic_body);
    let mut es = EventSource::new(req)?;
    let model = body.model.clone();

    let stream = async_stream::stream! {
        let mut tool_call_buffer = String::new();
        let mut in_tool_call = false;
        let mut tool_call_index = 0u32;
        // 累计 token 统计
        let mut prompt_tokens: u64 = 0;
        let mut completion_tokens: u64 = 0;
        
        for chunk in AnthropicMapper::initial_chunks() {
            yield Ok::<_, std::convert::Infallible>(Event::default().data(chunk.data));
        }

        while let Some(event) = es.next().await {
            match event {
                Ok(ReqwestEvent::Open) => continue,
                Ok(ReqwestEvent::Message(message)) => {
                    let text = message.data.clone();
                    // 尝试解析 Anthropic usage 字段（出现在 message_start 和 message_delta 事件）
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                        // message_start: { "message": { "usage": { "input_tokens": N, "output_tokens": M } } }
                        if let Some(usage) = json_val.pointer("/message/usage") {
                            prompt_tokens = usage["input_tokens"].as_u64().unwrap_or(prompt_tokens);
                            completion_tokens = usage["output_tokens"].as_u64().unwrap_or(completion_tokens);
                        }
                        // message_delta: { "usage": { "output_tokens": N } }
                        if let Some(usage) = json_val.get("usage") {
                            if let Some(v) = usage["output_tokens"].as_u64() {
                                completion_tokens = v;
                            }
                        }
                    }
                    if let Ok(chunks) = AnthropicMapper::map_delta(&model, text, false, &mut tool_call_buffer, &mut in_tool_call, &mut tool_call_index).await {
                        for chunk in chunks {
                            yield Ok(Event::default().data(chunk.data));
                        }
                    }
                }
                Err(e) => {
                    if let reqwest_eventsource::Error::StreamEnded = e {
                        if let Ok(chunks) = AnthropicMapper::map_delta(&model, String::new(), true, &mut tool_call_buffer, &mut in_tool_call, &mut tool_call_index).await {
                            for chunk in chunks {
                                yield Ok(Event::default().data(chunk.data));
                            }
                        }
                        // 写入 Token 审计记录
                        audit.write_usage(prompt_tokens, completion_tokens, prompt_tokens + completion_tokens);
                        yield Ok(Event::default().data("[DONE]"));
                        break;
                    }
                    tracing::error!("Anthropic SSE error: {}", e);
                    break;
                }
            }
        }
    };
    
    Ok(Sse::new(stream).into_response())
}

async fn handle_openai_compatible_stream(
    client: &reqwest::Client,
    secret: &str,
    base_url: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
    audit: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    use crate::proxy::mappers::{ProtocolMapper, openai::OpenAiMapper};

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let mut request = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", secret))
        .header("Content-Type", "application/json");

    if let Some(dp) = device_profile {
        request = request
            .header("x-machine-id", &dp.machine_id)
            .header("x-mac-machine-id", &dp.mac_machine_id)
            .header("x-dev-device-id", &dp.dev_device_id)
            .header("x-sqm-id", &dp.sqm_id);
    }

    let req = request.json(body);
    let mut es = EventSource::new(req)?;
    let model = body.model.clone();

    let stream = async_stream::stream! {
        let mut tool_call_buffer = String::new();
        let mut in_tool_call = false;
        let mut tool_call_index = 0u32;
        let mut prompt_tokens: u64 = 0;
        let mut completion_tokens: u64 = 0;
        let mut total_tokens: u64 = 0;
        
        for chunk in OpenAiMapper::initial_chunks() {
            yield Ok::<_, std::convert::Infallible>(Event::default().data(chunk.data));
        }

        while let Some(event) = es.next().await {
            match event {
                Ok(ReqwestEvent::Open) => continue,
                Ok(ReqwestEvent::Message(message)) => {
                    if message.data == "[DONE]" {
                        if let Ok(chunks) = OpenAiMapper::map_delta(&model, String::new(), true, &mut tool_call_buffer, &mut in_tool_call, &mut tool_call_index).await {
                            for chunk in chunks {
                                yield Ok(Event::default().data(chunk.data));
                            }
                        }
                        // 写入 Token 审计记录
                        audit.write_usage(prompt_tokens, completion_tokens, total_tokens);
                        yield Ok(Event::default().data("[DONE]"));
                        break;
                    }
                    
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&message.data) {
                        // 拦截末尾 usage 统计帧（部分 OpenAI 兼容端点在流中发送）
                        if let Some(usage) = json_val.get("usage") {
                            prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(prompt_tokens);
                            completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(completion_tokens);
                            total_tokens = usage["total_tokens"].as_u64().unwrap_or(total_tokens);
                        }
                        if let Some(content) = json_val["choices"][0]["delta"]["content"].as_str() {
                            if let Ok(chunks) = OpenAiMapper::map_delta(&model, content.to_string(), false, &mut tool_call_buffer, &mut in_tool_call, &mut tool_call_index).await {
                                for chunk in chunks {
                                    yield Ok(Event::default().data(chunk.data));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("OpenAI SSE error: {}", e);
                    break;
                }
            }
        }
    };
    
    Ok(Sse::new(stream).into_response())
}

/// 专为白嫖工具开的 IDE 特殊认证前置旁路接口
async fn forward_to_ide_bypass(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
) -> anyhow::Result<Value> {
    tracing::info!("🔗 [专属旁路通信网] 目标直击 IDE 池化云接口...");
    // 未来可接基于 Cursor 或 Copilot 白嫖接口的专属协议，此阶段暂时当做 Claude Code Auth 接口兼容
    forward_to_anthropic(client, secret, body, device_profile).await
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
