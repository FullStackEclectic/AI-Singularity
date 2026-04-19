use crate::db::Database;
use crate::proxy::converter::{
    anthropic_to_openai_response, gemini_to_openai_response, openai_to_anthropic,
    openai_to_gemini, OpenAIRequest,
};
use axum::response::{
    sse::{Event, Sse},
    IntoResponse,
};
use futures::stream::StreamExt;
use reqwest_eventsource::{Event as ReqwestEvent, EventSource};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) struct AuditContext {
    db: Arc<Database>,
    key_id: String,
    platform: String,
    model: String,
    client_app: String,
    user_token_id: Option<String>,
}

impl AuditContext {
    pub(super) fn new(
        db: Arc<Database>,
        key_id: String,
        platform: String,
        model: String,
        client_app: String,
        user_token_id: Option<String>,
    ) -> Self {
        Self {
            db,
            key_id,
            platform,
            model,
            client_app,
            user_token_id,
        }
    }

    pub(super) fn write_usage(&self, prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) {
        let cost = crate::services::pricing::PricingEngine::calculate_cost(
            &self.model,
            prompt_tokens,
            completion_tokens,
        );
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
            ],
        );

        if let Some(ut_id) = &self.user_token_id {
            let token_service = crate::services::user_token::UserTokenService::new(&self.db);
            let _ = token_service.increment_token_usage(ut_id, 1, total_tokens as i64);
        }
    }
}

pub(super) async fn forward_to_openai_compatible(
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

    if let Some(dp) = device_profile {
        tracing::info!(
            "🛡️ 启用指纹降维伪装 [IDE]: 正在向游离服务点重写物理设备信息 - MachineID: {}",
            dp.machine_id
        );
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

pub(super) async fn forward_to_anthropic(
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
        tracing::info!(
            "🛡️ 启用指纹降维伪装 [Anthropic]: 正在向服务器重写物理指纹 - MachineID: {}",
            dp.machine_id
        );
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

pub(super) async fn handle_anthropic_stream(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
    audit: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    use crate::proxy::mappers::{anthropic::AnthropicMapper, ProtocolMapper};

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
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(usage) = json_val.pointer("/message/usage") {
                            prompt_tokens = usage["input_tokens"].as_u64().unwrap_or(prompt_tokens);
                            completion_tokens = usage["output_tokens"].as_u64().unwrap_or(completion_tokens);
                        }
                        if let Some(usage) = json_val.get("usage") {
                            if let Some(v) = usage["output_tokens"].as_u64() {
                                completion_tokens = v;
                            }
                        }
                    }
                    if let Ok(chunks) = AnthropicMapper::map_delta(
                        &model,
                        text,
                        false,
                        &mut tool_call_buffer,
                        &mut in_tool_call,
                        &mut tool_call_index,
                    )
                    .await
                    {
                        for chunk in chunks {
                            yield Ok(Event::default().data(chunk.data));
                        }
                    }
                }
                Err(e) => {
                    if let reqwest_eventsource::Error::StreamEnded = e {
                        if let Ok(chunks) = AnthropicMapper::map_delta(
                            &model,
                            String::new(),
                            true,
                            &mut tool_call_buffer,
                            &mut in_tool_call,
                            &mut tool_call_index,
                        )
                        .await
                        {
                            for chunk in chunks {
                                yield Ok(Event::default().data(chunk.data));
                            }
                        }
                        audit.write_usage(
                            prompt_tokens,
                            completion_tokens,
                            prompt_tokens + completion_tokens,
                        );
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

pub(super) async fn handle_openai_compatible_stream(
    client: &reqwest::Client,
    secret: &str,
    base_url: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
    audit: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    use crate::proxy::mappers::{openai::OpenAiMapper, ProtocolMapper};

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
                        if let Ok(chunks) = OpenAiMapper::map_delta(
                            &model,
                            String::new(),
                            true,
                            &mut tool_call_buffer,
                            &mut in_tool_call,
                            &mut tool_call_index,
                        )
                        .await
                        {
                            for chunk in chunks {
                                yield Ok(Event::default().data(chunk.data));
                            }
                        }
                        audit.write_usage(prompt_tokens, completion_tokens, total_tokens);
                        yield Ok(Event::default().data("[DONE]"));
                        break;
                    }

                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&message.data) {
                        if let Some(usage) = json_val.get("usage") {
                            prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(prompt_tokens);
                            completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(completion_tokens);
                            total_tokens = usage["total_tokens"].as_u64().unwrap_or(total_tokens);
                        }
                        if let Some(content) = json_val["choices"][0]["delta"]["content"].as_str() {
                            if let Ok(chunks) = OpenAiMapper::map_delta(
                                &model,
                                content.to_string(),
                                false,
                                &mut tool_call_buffer,
                                &mut in_tool_call,
                                &mut tool_call_index,
                            )
                            .await
                            {
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

pub(super) async fn forward_to_ide_bypass(
    client: &reqwest::Client,
    secret: &str,
    body: &OpenAIRequest,
    device_profile: Option<&crate::models::DeviceProfile>,
) -> anyhow::Result<Value> {
    tracing::info!("🔗 [专属旁路通信网] 目标直击 IDE 池化云接口...");
    forward_to_anthropic(client, secret, body, device_profile).await
}

pub(super) async fn forward_to_gemini(
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

pub(super) async fn forward_to_gemini_imagen3(
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
