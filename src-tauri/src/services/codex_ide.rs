use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use base64::Engine;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use serde_json::{Map, Value};

const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const CODEX_USAGE_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_ACCOUNT_CHECK_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/accounts/check";

#[derive(Debug, Deserialize)]
struct CodexTokenRefreshResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<u64>,
    token_type: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WindowInfo {
    #[serde(rename = "used_percent")]
    used_percent: Option<i32>,
    #[serde(rename = "limit_window_seconds")]
    limit_window_seconds: Option<i64>,
    #[serde(rename = "reset_after_seconds")]
    reset_after_seconds: Option<i64>,
    #[serde(rename = "reset_at")]
    reset_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RateLimitInfo {
    #[serde(rename = "primary_window")]
    primary_window: Option<WindowInfo>,
    #[serde(rename = "secondary_window")]
    secondary_window: Option<WindowInfo>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[serde(rename = "plan_type")]
    plan_type: Option<String>,
    #[serde(rename = "rate_limit")]
    rate_limit: Option<RateLimitInfo>,
}

struct CodexProfile {
    account_name: Option<String>,
    account_structure: Option<String>,
    account_id: Option<String>,
}

pub struct CodexIdeService;

impl CodexIdeService {
    pub async fn refresh_all_accounts(db: &Database) -> Result<usize, String> {
        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case("codex"))
            .collect::<Vec<_>>();

        let mut success = 0usize;
        for account in accounts {
            if Self::refresh_account(db, &account.id).await.is_ok() {
                success += 1;
            }
        }
        Ok(success)
    }

    pub async fn refresh_account(db: &Database, account_id: &str) -> Result<IdeAccount, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Codex 账号不存在".to_string())?;

        if !account.origin_platform.eq_ignore_ascii_case("codex") {
            return Err("当前账号不是 Codex 类型".to_string());
        }

        let mut meta = parse_meta_json_object(account.meta_json.as_deref());
        let auth_mode = get_meta_string(&meta, "auth_mode").unwrap_or_else(|| "oauth".to_string());
        if auth_mode.eq_ignore_ascii_case("apikey") {
            return Err("Codex API Key 模式账号暂不支持刷新订阅配额".to_string());
        }

        if account.token.access_token.trim().is_empty() {
            return Err("Codex 账号缺少 access_token".to_string());
        }

        if is_token_expired(&account.token.access_token) {
            Self::refresh_tokens(&mut account, &mut meta, "Token 已过期").await?;
        }

        let mut quota_result = Self::fetch_quota(&account, &meta).await;
        if quota_result
            .as_ref()
            .err()
            .is_some_and(|err| should_force_refresh_token(err))
        {
            Self::refresh_tokens(&mut account, &mut meta, "Codex 配额请求要求刷新 Token").await?;
            quota_result = Self::fetch_quota(&account, &meta).await;
        }

        let (quota_json, plan_type) = quota_result?;
        if let Some(plan_type) = normalize_string(plan_type) {
            meta.insert("plan_type".to_string(), Value::String(plan_type));
        }

        let mut profile_result = Self::fetch_remote_profile(&account, &meta).await;
        if profile_result
            .as_ref()
            .err()
            .is_some_and(|err| should_force_refresh_token(err))
        {
            Self::refresh_tokens(&mut account, &mut meta, "Codex 资料请求要求刷新 Token").await?;
            profile_result = Self::fetch_remote_profile(&account, &meta).await;
        }

        if let Ok(profile) = profile_result {
            if let Some(account_name) = normalize_string(profile.account_name) {
                meta.insert("account_name".to_string(), Value::String(account_name));
            }
            if let Some(account_structure) = normalize_string(profile.account_structure) {
                meta.insert(
                    "account_structure".to_string(),
                    Value::String(account_structure),
                );
            }
            if let Some(account_id) = normalize_string(profile.account_id) {
                meta.insert("account_id".to_string(), Value::String(account_id));
            }
        }

        if let Some(account_id) =
            normalize_string(get_meta_string(&meta, "account_id").or_else(|| {
                extract_chatgpt_account_id_from_access_token(&account.token.access_token)
            }))
        {
            meta.insert("account_id".to_string(), Value::String(account_id));
        }
        if let Some(organization_id) =
            normalize_string(get_meta_string(&meta, "organization_id").or_else(|| {
                extract_chatgpt_organization_id_from_access_token(&account.token.access_token)
            }))
        {
            meta.insert(
                "organization_id".to_string(),
                Value::String(organization_id),
            );
        }
        if let Some(user_id) = normalize_string(
            get_meta_string(&meta, "user_id")
                .or_else(|| decode_jwt_claim(&account.token.access_token, "sub"))
                .or_else(|| {
                    get_meta_string(&meta, "id_token")
                        .and_then(|id_token| decode_jwt_claim(&id_token, "sub"))
                }),
        ) {
            meta.insert("user_id".to_string(), Value::String(user_id));
        }
        if let Some(email) = normalize_string(
            decode_jwt_claim(&account.token.access_token, "email").or_else(|| {
                get_meta_string(&meta, "id_token")
                    .and_then(|id_token| decode_jwt_claim(&id_token, "email"))
            }),
        ) {
            account.email = email;
        }

        meta.insert(
            "last_refresh".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );

        account.status = AccountStatus::Active;
        account.disabled_reason = None;
        account.quota_json = Some(quota_json.to_string());
        account.meta_json = Some(Value::Object(meta).to_string());
        account.updated_at = Utc::now();
        account.last_used = Utc::now();

        db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
        Ok(account)
    }

    pub fn update_api_key_credentials(
        db: &Database,
        account_id: &str,
        api_key: &str,
        api_base_url: Option<&str>,
    ) -> Result<IdeAccount, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Codex 账号不存在".to_string())?;

        if !account.origin_platform.eq_ignore_ascii_case("codex") {
            return Err("当前账号不是 Codex 类型".to_string());
        }

        let normalized_api_key = normalize_api_key(api_key)?;
        let normalized_base_url = normalize_api_base_url(api_base_url)?;
        let mut meta = parse_meta_json_object(account.meta_json.as_deref());

        meta.insert("auth_mode".to_string(), Value::String("apikey".to_string()));
        meta.insert(
            "openai_api_key".to_string(),
            Value::String(normalized_api_key.clone()),
        );
        meta.insert(
            "last_refresh".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );
        match normalized_base_url {
            Some(base_url) => {
                meta.insert("api_base_url".to_string(), Value::String(base_url));
            }
            None => {
                meta.remove("api_base_url");
            }
        }
        meta.insert("plan_type".to_string(), Value::String("API_KEY".to_string()));

        account.token.access_token = String::new();
        account.token.refresh_token = "missing".to_string();
        account.token.expires_in = 0;
        account.token.token_type = "Bearer".to_string();
        account.token.updated_at = Utc::now();
        account.status = AccountStatus::Active;
        account.disabled_reason = None;
        account.quota_json = None;
        account.meta_json = Some(Value::Object(meta).to_string());
        account.updated_at = Utc::now();
        account.last_used = Utc::now();

        if account.email.trim().is_empty() || account.email.contains("@oauth.local") {
            let suffix = normalized_api_key
                .chars()
                .rev()
                .take(6)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            account.email = format!("codex-apikey-{}@local", suffix);
        }

        db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
        Ok(account)
    }

    async fn refresh_tokens(
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

    async fn fetch_quota(
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

    async fn fetch_remote_profile(
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

fn parse_meta_json_object(raw: Option<&str>) -> Map<String, Value> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn get_meta_string(meta: &Map<String, Value>, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn normalize_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_api_key(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Codex API Key 不能为空".to_string());
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err("Codex API Key 不能是 URL，请检查是否填反".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_api_base_url(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| "Codex API Base URL 格式无效，请输入完整的 http:// 或 https:// 地址".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Codex API Base URL 仅支持 http 或 https 协议".to_string());
    }
    Ok(Some(trimmed.trim_end_matches('/').to_string()))
}

fn decode_jwt_payload(token: &str) -> Option<Value> {
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let payload_b64 = parts[1].replace('-', "+").replace('_', "/");
    let padded = match payload_b64.len() % 4 {
        2 => format!("{}==", payload_b64),
        3 => format!("{}=", payload_b64),
        _ => payload_b64,
    };
    let payload = base64::engine::general_purpose::STANDARD
        .decode(padded)
        .ok()?;
    serde_json::from_slice::<Value>(&payload).ok()
}

fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
    decode_jwt_payload(token)?
        .get(claim)?
        .as_str()
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
}

fn decode_any_claim(token: &str, claims: &[&str]) -> Option<String> {
    for claim in claims {
        if let Some(value) = decode_jwt_claim(token, claim) {
            return Some(value);
        }
    }
    None
}

fn is_token_expired(access_token: &str) -> bool {
    let exp = decode_jwt_payload(access_token)
        .and_then(|payload| payload.get("exp").and_then(|value| value.as_i64()));
    let Some(exp) = exp else {
        return true;
    };
    exp < Utc::now().timestamp() + 60
}

fn should_force_refresh_token(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("token_invalidated")
        || lower.contains("authentication token has been invalidated")
        || lower.contains("401 unauthorized")
        || lower.contains("status=401")
        || lower.contains("错误 401")
}

fn extract_chatgpt_account_id_from_access_token(access_token: &str) -> Option<String> {
    decode_any_claim(
        access_token,
        &["chatgpt_account_id", "account_id", "workspace_id"],
    )
}

fn extract_chatgpt_organization_id_from_access_token(access_token: &str) -> Option<String> {
    decode_any_claim(
        access_token,
        &[
            "chatgpt_organization_id",
            "chatgpt_org_id",
            "organization_id",
            "org_id",
            "workspace_id",
        ],
    )
}

fn resolve_codex_account_id(account: &IdeAccount, meta: &Map<String, Value>) -> Option<String> {
    normalize_string(
        get_meta_string(meta, "account_id")
            .or_else(|| extract_chatgpt_account_id_from_access_token(&account.token.access_token)),
    )
}

fn normalize_remaining_percentage(window: &WindowInfo) -> i32 {
    100 - window.used_percent.unwrap_or(0).clamp(0, 100)
}

fn normalize_window_minutes(window: &WindowInfo) -> Option<i64> {
    let seconds = window.limit_window_seconds?;
    if seconds <= 0 {
        return None;
    }
    Some((seconds + 59) / 60)
}

fn normalize_reset_time(window: &WindowInfo) -> Option<i64> {
    if let Some(reset_at) = window.reset_at {
        return Some(reset_at);
    }
    let reset_after_seconds = window.reset_after_seconds?;
    if reset_after_seconds < 0 {
        return None;
    }
    Some(Utc::now().timestamp() + reset_after_seconds)
}

fn parse_remote_profile(
    payload: &Value,
    account: &IdeAccount,
    meta: &Map<String, Value>,
) -> CodexProfile {
    let records = collect_account_records(payload);
    if records.is_empty() {
        return CodexProfile {
            account_name: None,
            account_structure: None,
            account_id: None,
        };
    }

    let ordering_first_id = payload
        .get("account_ordering")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let expected_account_id = resolve_codex_account_id(account, meta);
    let expected_org_id =
        normalize_string(get_meta_string(meta, "organization_id").or_else(|| {
            extract_chatgpt_organization_id_from_access_token(&account.token.access_token)
        }));

    let selected = records
        .iter()
        .find(|record| {
            expected_account_id.as_ref().is_some_and(|expected| {
                extract_account_record_field(
                    record,
                    &["id", "account_id", "chatgpt_account_id", "workspace_id"],
                )
                .is_some_and(|candidate| candidate == *expected)
            })
        })
        .cloned()
        .or_else(|| {
            records
                .iter()
                .find(|record| {
                    ordering_first_id.as_ref().is_some_and(|expected| {
                        extract_account_record_field(
                            record,
                            &["id", "account_id", "chatgpt_account_id", "workspace_id"],
                        )
                        .is_some_and(|candidate| candidate == *expected)
                    })
                })
                .cloned()
        })
        .or_else(|| {
            records
                .iter()
                .find(|record| {
                    expected_org_id.as_ref().is_some_and(|expected| {
                        extract_account_record_field(
                            record,
                            &["organization_id", "org_id", "workspace_id"],
                        )
                        .is_some_and(|candidate| candidate == *expected)
                    })
                })
                .cloned()
        })
        .unwrap_or_else(|| records[0].clone());

    CodexProfile {
        account_name: extract_account_record_field(
            &selected,
            &[
                "name",
                "display_name",
                "account_name",
                "organization_name",
                "workspace_name",
                "title",
            ],
        ),
        account_structure: extract_account_record_field(
            &selected,
            &[
                "structure",
                "account_structure",
                "kind",
                "type",
                "account_type",
            ],
        ),
        account_id: extract_account_record_field(
            &selected,
            &["id", "account_id", "chatgpt_account_id", "workspace_id"],
        ),
    }
}

fn collect_account_records(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    collect_account_records_inner(value, &mut out);
    out
}

fn collect_account_records_inner(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_account_records_inner(item, out);
            }
        }
        Value::Object(map) => {
            if looks_like_account_record(map) {
                out.push(value.clone());
            }
            for nested in map.values() {
                collect_account_records_inner(nested, out);
            }
        }
        _ => {}
    }
}

fn looks_like_account_record(map: &Map<String, Value>) -> bool {
    let id_like = ["id", "account_id", "chatgpt_account_id", "workspace_id"]
        .iter()
        .any(|key| map.get(*key).and_then(|value| value.as_str()).is_some());
    let name_like = [
        "name",
        "display_name",
        "account_name",
        "organization_name",
        "workspace_name",
        "title",
    ]
    .iter()
    .any(|key| map.get(*key).and_then(|value| value.as_str()).is_some());
    id_like || name_like
}

fn extract_account_record_field(record: &Value, keys: &[&str]) -> Option<String> {
    let object = record.as_object()?;
    for key in keys {
        if let Some(value) = object.get(*key).and_then(|value| value.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}
