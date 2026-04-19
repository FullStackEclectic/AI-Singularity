use super::helpers::{construct_pollinations_response, extract_image_prompt};
use super::transport::forward_to_gemini_imagen3;
use super::ProxyState;
use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

pub(super) struct ProxyRequestContext {
    pub current_user_token: Option<String>,
    pub client_app: String,
    pub target_scope: crate::models::TokenScope,
}

pub(super) async fn resolve_request_context(
    state: &ProxyState,
    client_ip: &ConnectInfo<std::net::SocketAddr>,
    headers: &HeaderMap,
) -> Result<ProxyRequestContext, Response> {
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let incoming_ip = client_ip.0.ip().to_string();
    let mut current_user_token = None;
    let mut target_scope = default_token_scope();

    if let Some(auth) = auth_header {
        if auth.starts_with("Bearer sk-ag-") {
            let token_str = &auth[7..];
            let token_service = crate::services::user_token::UserTokenService::new(&state.db);

            match token_service.get_token_by_str(token_str) {
                Ok(Some(ut)) => {
                    if !ut.enabled {
                        return Err((
                            StatusCode::FORBIDDEN,
                            Json(json!({"error": {"message": "Token 已被主脑冻结"}})),
                        )
                            .into_response());
                    }

                    let now = chrono::Utc::now().timestamp();
                    if ut.expires_type == "absolute" {
                        if let Some(exp) = ut.expires_at {
                            if now > exp {
                                return Err((
                                    StatusCode::FORBIDDEN,
                                    Json(json!({"error": {"message": "Token 绝对有效期已过"}})),
                                )
                                    .into_response());
                            }
                        }
                    } else if ut.expires_type == "relative" {
                        if let Some(exp) = ut.expires_at {
                            if now > ut.created_at + exp {
                                return Err((
                                    StatusCode::FORBIDDEN,
                                    Json(json!({"error": {"message": "Token 相对配额期已过"}})),
                                )
                                    .into_response());
                            }
                        }
                    }

                    if let (Some(cs), Some(ce)) = (&ut.curfew_start, &ut.curfew_end) {
                        let cur_time = chrono::Utc::now().format("%H:%M").to_string();
                        if cs < ce {
                            if cur_time < *cs || cur_time > *ce {
                                return Err((StatusCode::FORBIDDEN, Json(json!({"error": {"message": format!("当前处于系统宵禁时段外 (允许: {}-{})", cs, ce)}}))).into_response());
                            }
                        } else if cur_time < *cs && cur_time > *ce {
                            return Err((StatusCode::FORBIDDEN, Json(json!({"error": {"message": format!("当前处于跨夜系统宵禁时段外 (允许: {}-{})", cs, ce)}}))).into_response());
                        }
                    }

                    if let crate::proxy::security::SecurityAction::Deny(reason) =
                        crate::proxy::security::SecurityShield::verify_max_ips(
                            &ut.id,
                            &incoming_ip,
                            ut.max_ips,
                        )
                    {
                        spawn_access_log(
                            state.db.clone(),
                            incoming_ip.clone(),
                            Some(ut.id.clone()),
                            "deny",
                            Some(reason.clone()),
                        );
                        return Err((
                            StatusCode::TOO_MANY_REQUESTS,
                            Json(json!({"error": {"message": reason}})),
                        )
                            .into_response());
                    }

                    target_scope = ut.parse_scope();
                    current_user_token = Some(ut.id);
                }
                _ => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": {"message": "无效的 AI Singularity 下发 Token"}})),
                    )
                        .into_response());
                }
            }
        }
    }

    if current_user_token.is_none() && !client_ip.0.ip().is_loopback() {
        spawn_access_log(
            state.db.clone(),
            incoming_ip.clone(),
            None,
            "deny",
            Some("外部无鉴权访问".to_string()),
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": {"message": "外部 IP 必须携带合法的网关分发 Token (sk-ag-...)"}})),
        )
            .into_response());
    }

    if let crate::proxy::security::SecurityAction::Deny(reason) =
        crate::proxy::security::SecurityShield::verify_ip_rule(&incoming_ip)
    {
        spawn_access_log(
            state.db.clone(),
            incoming_ip.clone(),
            current_user_token.clone(),
            "blacklisted",
            Some(reason.clone()),
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": {"message": reason}})),
        )
            .into_response());
    }

    if let crate::proxy::security::SecurityAction::Deny(reason) =
        crate::proxy::security::SecurityShield::check_rate_limit(
            &incoming_ip,
            current_user_token.as_deref(),
        )
    {
        spawn_access_log(
            state.db.clone(),
            incoming_ip.clone(),
            current_user_token.clone(),
            "rate_limit",
            Some(reason.clone()),
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": {"message": reason}})),
        )
            .into_response());
    }

    if !client_ip.0.ip().is_loopback() {
        spawn_access_log(
            state.db.clone(),
            incoming_ip,
            current_user_token.clone(),
            "allow",
            None,
        );
    }

    let client_app = headers
        .get("x-client-app")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("user-agent").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();

    Ok(ProxyRequestContext {
        current_user_token,
        client_app,
        target_scope,
    })
}

pub(super) async fn handle_image_intercept(
    state: &ProxyState,
    body: &crate::proxy::converter::OpenAIRequest,
    target_scope: &crate::models::TokenScope,
) -> Option<Response> {
    let img_prompt = extract_image_prompt(&body.messages)?;

    tracing::info!("🎨 检测到绘图指令跨模态截留: {}", img_prompt);
    if let Some(target) = state.router.pick_best_key(Some("gemini"), target_scope) {
        match forward_to_gemini_imagen3(&state.http_client, &target.secret, &img_prompt).await {
            Ok(resp) => return Some((StatusCode::OK, Json(resp)).into_response()),
            Err(e) => tracing::warn!("Gemini Imagen 3 失败: {}, 降级为外部公开代理绘图", e),
        }
    }

    Some((
        StatusCode::OK,
        Json(construct_pollinations_response(&img_prompt)),
    )
        .into_response())
}

fn default_token_scope() -> crate::models::TokenScope {
    crate::models::TokenScope {
        scope: "global".to_string(),
        desc: None,
        channels: vec![],
        tags: vec![],
        single_account: None,
    }
}

fn spawn_access_log(
    db: Arc<crate::db::Database>,
    ip: String,
    user_token_id: Option<String>,
    action: &'static str,
    reason: Option<String>,
) {
    tokio::spawn(async move {
        let svc = crate::services::security_db::SecurityDbService::new(&db);
        let _ = svc.log_access(
            &ip,
            "/v1/chat/completions",
            user_token_id.as_deref(),
            action,
            reason.as_deref(),
        );
    });
}
