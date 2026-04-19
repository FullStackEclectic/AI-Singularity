use super::helpers::{get_fallback_model, get_platform_for_model, platform_base_url};
use super::request::ProxyRequestContext;
use super::transport::{
    forward_to_anthropic, forward_to_gemini, forward_to_ide_bypass,
    forward_to_openai_compatible, handle_anthropic_stream, handle_openai_compatible_stream,
    AuditContext,
};
use super::{ForwardTarget, ProxyState};
use crate::models::Platform;
use crate::proxy::converter::OpenAIRequest;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub(super) async fn forward_chat_completion(
    state: &ProxyState,
    request_ctx: &ProxyRequestContext,
    body: &mut OpenAIRequest,
    mut current_model: String,
    is_stream: bool,
) -> Response {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let target = match pick_forward_target(state, request_ctx, &current_model, attempts) {
            PickTargetOutcome::Target(target) => target,
            PickTargetOutcome::FallbackModel(fallback) => {
                current_model = fallback;
                body.model = current_model.clone();
                continue;
            }
            PickTargetOutcome::Unavailable => return no_keys_response(request_ctx, &current_model),
        };

        let audit_ctx = AuditContext::new(
            state.db.clone(),
            target.key_id.clone(),
            format!("{:?}", target.platform),
            current_model.clone(),
            request_ctx.client_app.clone(),
            request_ctx.current_user_token.clone(),
        );

        let response_or_stream = if is_stream {
            forward_stream_request(state, body, &target, audit_ctx).await
        } else {
            forward_json_request(state, body, &target, audit_ctx).await
        };

        match response_or_stream {
            Ok(axum_resp) => {
                if attempts > 1 {
                    tracing::info!("✅ 动态灾备成功：已通过回退模型 {} 完成调用", current_model);
                }
                return axum_resp.into_response();
            }
            Err(err) => {
                let err_str = err.to_string();
                mark_target_failure(state, &target, &err_str);

                if should_retry_or_fallback(&err_str) {
                    let max_retries = max_retry_attempts();
                    if attempts < max_retries {
                        tracing::warn!(
                            "⚠️ 节点 [{}] (平台: {:?}) 发生熔断 ({})，触发毫秒级切流 (已尝试: {}/{})",
                            target.key_id,
                            target.platform,
                            err_str.lines().next().unwrap_or(&err_str),
                            attempts,
                            max_retries
                        );
                        continue;
                    }

                    if let Some(fallback) = get_fallback_model(&current_model) {
                        tracing::warn!(
                            "⚠️ 连续熔断！模型 {} 不可用，降灾备份至 {}",
                            current_model,
                            fallback
                        );
                        current_model = fallback.to_string();
                        body.model = current_model.clone();
                        attempts = 0;
                        continue;
                    }
                }

                return proxy_error_response(&err_str);
            }
        }
    }
}

fn pick_forward_target(
    state: &ProxyState,
    request_ctx: &ProxyRequestContext,
    current_model: &str,
    attempts: usize,
) -> PickTargetOutcome {
    let platform_str = get_platform_for_model(current_model);
    let target = state
        .router
        .pick_best_key(platform_str, &request_ctx.target_scope);

    if let Some(target) = target {
        return PickTargetOutcome::Target(ForwardTarget {
            secret: target.secret,
            base_url: target.base_url,
            platform: target.platform,
            key_id: target.key_id,
            device_profile: None,
        });
    }

    if attempts == 1 {
        if let Some(fallback) = get_fallback_model(current_model) {
            tracing::warn!(
                "⚠️ 模型 {} 无可用 Provider，正在智能回退至备选模型 {}",
                current_model,
                fallback
            );
            return PickTargetOutcome::FallbackModel(fallback.to_string());
        }
    }

    PickTargetOutcome::Unavailable
}

async fn forward_stream_request(
    state: &ProxyState,
    body: &OpenAIRequest,
    target: &ForwardTarget,
    audit_ctx: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    match &target.platform {
        Platform::Auth0IDE | Platform::Anthropic => {
            handle_anthropic_stream(
                &state.http_client,
                &target.secret,
                body,
                target.device_profile.as_ref(),
                audit_ctx,
            )
            .await
        }
        Platform::Gemini => {
            handle_openai_compatible_stream(
                &state.http_client,
                &target.secret,
                platform_base_url(&target.platform),
                body,
                target.device_profile.as_ref(),
                audit_ctx,
            )
            .await
        }
        _ => {
            let base = target
                .base_url
                .as_deref()
                .unwrap_or(platform_base_url(&target.platform));
            handle_openai_compatible_stream(
                &state.http_client,
                &target.secret,
                base,
                body,
                target.device_profile.as_ref(),
                audit_ctx,
            )
            .await
        }
    }
}

async fn forward_json_request(
    state: &ProxyState,
    body: &OpenAIRequest,
    target: &ForwardTarget,
    audit_ctx: AuditContext,
) -> anyhow::Result<axum::response::Response> {
    let result = match &target.platform {
        Platform::Auth0IDE => {
            forward_to_ide_bypass(
                &state.http_client,
                &target.secret,
                body,
                target.device_profile.as_ref(),
            )
            .await
        }
        Platform::Anthropic => {
            forward_to_anthropic(
                &state.http_client,
                &target.secret,
                body,
                target.device_profile.as_ref(),
            )
            .await
        }
        Platform::Gemini => forward_to_gemini(&state.http_client, &target.secret, body).await,
        _ => {
            let base = target
                .base_url
                .as_deref()
                .unwrap_or(platform_base_url(&target.platform));
            forward_to_openai_compatible(
                &state.http_client,
                &target.secret,
                base,
                body,
                target.device_profile.as_ref(),
            )
            .await
        }
    };

    result.map(|json_val| {
        if let Some(usage) = json_val.get("usage") {
            let prompt_tokens = usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let completion_tokens = usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total_tokens = usage
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            audit_ctx.write_usage(prompt_tokens, completion_tokens, total_tokens);
        }

        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_string(&json_val).unwrap(),
            ))
            .unwrap()
    })
}

fn mark_target_failure(state: &ProxyState, target: &ForwardTarget, err_str: &str) {
    if target.platform == Platform::Auth0IDE || target.device_profile.is_some() {
        if err_str.contains("401") || err_str.contains("403") {
            state
                .router
                .mark_key_status(&target.key_id, true, "forbidden");
        } else if err_str.contains("429") {
            state
                .router
                .mark_key_status(&target.key_id, true, "rate_limited");
        }
    } else if err_str.contains("401") || err_str.contains("403") {
        state
            .router
            .mark_key_status(&target.key_id, false, "invalid");
    } else if err_str.contains("429") {
        state
            .router
            .mark_key_status(&target.key_id, false, "rate_limit");
    }
}

fn should_retry_or_fallback(err_str: &str) -> bool {
    err_str.contains("429") || err_str.contains("50") || err_str.contains("timeout")
}

fn max_retry_attempts() -> usize {
    if let Ok(cfg) = crate::commands::proxy::ENGINE_CONFIG.read() {
        if cfg.circuit_breaker.enabled {
            cfg.circuit_breaker.backoff_steps.len()
        } else {
            1
        }
    } else {
        3
    }
}

fn no_keys_response(request_ctx: &ProxyRequestContext, current_model: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": {"message": format!("降维打击网关失败：无任何可用令牌满足当前 Scope [{}] 的分发规则，且无法进行灾备 {}", request_ctx.target_scope.scope, current_model), "type": "no_keys_in_scope"}})),
    )
        .into_response()
}

fn proxy_error_response(err_str: &str) -> Response {
    axum::response::Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_string(
                &json!({"error": {"message": format!("代理上游错误: {}", err_str), "type": "proxy_error"}}),
            )
            .unwrap(),
        ))
        .unwrap()
        .into_response()
}

enum PickTargetOutcome {
    Target(ForwardTarget),
    FallbackModel(String),
    Unavailable,
}
