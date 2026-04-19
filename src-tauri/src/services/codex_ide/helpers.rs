use crate::models::IdeAccount;
use base64::Engine;
use chrono::Utc;
use serde_json::{Map, Value};

pub(super) fn parse_meta_json_object(raw: Option<&str>) -> Map<String, Value> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

pub(super) fn get_meta_string(meta: &Map<String, Value>, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

pub(super) fn normalize_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn normalize_api_key(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Codex API Key 不能为空".to_string());
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err("Codex API Key 不能是 URL，请检查是否填反".to_string());
    }
    Ok(trimmed.to_string())
}

pub(super) fn normalize_api_base_url(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        "Codex API Base URL 格式无效，请输入完整的 http:// 或 https:// 地址".to_string()
    })?;
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

pub(super) fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
    decode_jwt_payload(token)?
        .get(claim)?
        .as_str()
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn decode_any_claim(token: &str, claims: &[&str]) -> Option<String> {
    for claim in claims {
        if let Some(value) = decode_jwt_claim(token, claim) {
            return Some(value);
        }
    }
    None
}

pub(super) fn is_token_expired(access_token: &str) -> bool {
    let exp = decode_jwt_payload(access_token)
        .and_then(|payload| payload.get("exp").and_then(|value| value.as_i64()));
    let Some(exp) = exp else {
        return true;
    };
    exp < Utc::now().timestamp() + 60
}

pub(super) fn should_force_refresh_token(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("token_invalidated")
        || lower.contains("authentication token has been invalidated")
        || lower.contains("401 unauthorized")
        || lower.contains("status=401")
        || lower.contains("错误 401")
}

pub(super) fn extract_chatgpt_account_id_from_access_token(
    access_token: &str,
) -> Option<String> {
    decode_any_claim(
        access_token,
        &["chatgpt_account_id", "account_id", "workspace_id"],
    )
}

pub(super) fn extract_chatgpt_organization_id_from_access_token(
    access_token: &str,
) -> Option<String> {
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

pub(super) fn resolve_codex_account_id(
    account: &IdeAccount,
    meta: &Map<String, Value>,
) -> Option<String> {
    normalize_string(
        get_meta_string(meta, "account_id")
            .or_else(|| extract_chatgpt_account_id_from_access_token(&account.token.access_token)),
    )
}
