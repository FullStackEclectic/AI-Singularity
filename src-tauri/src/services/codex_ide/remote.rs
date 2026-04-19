use super::helpers::{decode_any_claim, decode_jwt_claim, resolve_codex_account_id};
use super::profile::{
    normalize_remaining_percentage, normalize_reset_time, normalize_window_minutes,
    parse_remote_profile,
};
use super::{
    CodexIdeService, CodexProfile, CodexTokenRefreshResponse, UsageResponse,
};
use crate::models::IdeAccount;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::{Map, Value};

const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const CODEX_USAGE_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_ACCOUNT_CHECK_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/accounts/check";

impl CodexIdeService {
    pub(super) async fn refresh_tokens(
        account: &mut IdeAccount,
        meta: &mut Map<String, Value>,
        reason: &str,
    ) -> Result<(), String> {
        let refresh_token = account.token.refresh_token.trim();
        if refresh_token.is_empty() || refresh_token == "missing" {
            return Err(format!("{}，但账号缺少 refresh_token", reason));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client
            .post(CODEX_TOKEN_ENDPOINT)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", CODEX_CLIENT_ID),
            ])
            .send()
            .await
            .map_err(|e| format!("刷新 Codex access_token 请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "刷新 Codex access_token 失败: status={}, body={}",
                status, body
            ));
        }

        let payload = response
            .json::<CodexTokenRefreshResponse>()
            .await
            .map_err(|e| format!("解析 Codex access_token 刷新响应失败: {}", e))?;

        let access_token = payload.access_token.ok_or_else(|| {
            format!(
                "刷新 Codex access_token 响应异常: error={:?}, desc={:?}",
                payload.error, payload.error_description
            )
        })?;

        account.token.access_token = access_token;
        if let Some(refresh_token) = payload.refresh_token.filter(|s| !s.trim().is_empty()) {
            account.token.refresh_token = refresh_token;
        }
        if let Some(expires_in) = payload.expires_in {
            account.token.expires_in = expires_in;
        }
        if let Some(token_type) = payload.token_type.filter(|s| !s.trim().is_empty()) {
            account.token.token_type = token_type;
        }
        account.token.updated_at = Utc::now();

        if let Some(id_token) = payload.id_token.filter(|s| !s.trim().is_empty()) {
            meta.insert("id_token".to_string(), Value::String(id_token.clone()));
            if let Some(email) = decode_jwt_claim(&id_token, "email") {
                account.email = email;
            }
            if let Some(plan_type) =
                decode_any_claim(&id_token, &["chatgpt_plan_type", "plan_type"])
            {
                meta.insert("plan_type".to_string(), Value::String(plan_type));
            }
        }

        meta.insert(
            "last_refresh".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );
        Ok(())
    }

    pub(super) async fn fetch_quota(
        account: &IdeAccount,
        meta: &Map<String, Value>,
    ) -> Result<(Value, Option<String>), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", account.token.access_token))
                .map_err(|e| format!("构建 Authorization 头失败: {}", e))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(account_id) = resolve_codex_account_id(account, meta) {
            headers.insert(
                "ChatGPT-Account-Id",
                HeaderValue::from_str(&account_id)
                    .map_err(|e| format!("构建 ChatGPT-Account-Id 头失败: {}", e))?,
            );
        }

        let response = client
            .get(CODEX_USAGE_ENDPOINT)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求 Codex 配额失败: {}", e))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<empty-body>".to_string());
        if !status.is_success() {
            return Err(format!("Codex 配额接口返回错误 {}: {}", status, body));
        }

        let usage = serde_json::from_str::<UsageResponse>(&body)
            .map_err(|e| format!("解析 Codex 配额响应失败: {}", e))?;
        let raw_value = serde_json::from_str::<Value>(&body).unwrap_or(Value::Null);
        let rate_limit = usage.rate_limit.as_ref();
        let primary = rate_limit.and_then(|item| item.primary_window.as_ref());
        let secondary = rate_limit.and_then(|item| item.secondary_window.as_ref());

        let quota = serde_json::json!({
            "provider": "codex",
            "synced_at": Utc::now().to_rfc3339(),
            "plan_type": usage.plan_type,
            "hourly_percentage": primary.map(normalize_remaining_percentage),
            "hourly_reset_time": primary.and_then(normalize_reset_time),
            "hourly_window_minutes": primary.and_then(normalize_window_minutes),
            "weekly_percentage": secondary.map(normalize_remaining_percentage),
            "weekly_reset_time": secondary.and_then(normalize_reset_time),
            "weekly_window_minutes": secondary.and_then(normalize_window_minutes),
            "raw": raw_value,
        });

        Ok((quota, usage.plan_type))
    }

    pub(super) async fn fetch_remote_profile(
        account: &IdeAccount,
        meta: &Map<String, Value>,
    ) -> Result<CodexProfile, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", account.token.access_token))
                .map_err(|e| format!("构建 Authorization 头失败: {}", e))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(account_id) = resolve_codex_account_id(account, meta) {
            headers.insert(
                "ChatGPT-Account-Id",
                HeaderValue::from_str(&account_id)
                    .map_err(|e| format!("构建 ChatGPT-Account-Id 头失败: {}", e))?,
            );
        }

        let response = client
            .get(CODEX_ACCOUNT_CHECK_ENDPOINT)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求 Codex 账号资料失败: {}", e))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<empty-body>".to_string());
        if !status.is_success() {
            return Err(format!("Codex 账号资料接口返回错误 {}: {}", status, body));
        }

        let payload = serde_json::from_str::<Value>(&body)
            .map_err(|e| format!("解析 Codex 账号资料响应失败: {}", e))?;
        Ok(parse_remote_profile(&payload, account, meta))
    }
}
