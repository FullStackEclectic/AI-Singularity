use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::Database;
use crate::models::IdeAccount;

const CODEBUDDY_CN_API_ENDPOINT: &str = "https://www.codebuddy.cn";
const MAX_RETRY_FAILED_TIMES: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IdeStatusActionResult {
    pub account_id: String,
    pub platform: String,
    pub action: String,
    pub success: bool,
    pub message: String,
    pub reward: Option<Value>,
    pub next_checkin_in: Option<i64>,
    pub attempts: u32,
    pub retried: bool,
    pub retryable: bool,
    pub executed_at: String,
}

#[derive(Debug, Clone)]
struct DailyCheckinResponse {
    success: bool,
    message: Option<String>,
    reward: Option<Value>,
    next_checkin_in: Option<i64>,
    retryable: bool,
}

#[derive(Debug, Clone, Default)]
struct PlatformAuthContext {
    uid: Option<String>,
    enterprise_id: Option<String>,
    domain: Option<String>,
}

pub struct PlatformStatusActionService;

impl PlatformStatusActionService {
    pub async fn execute(
        db: &Database,
        account_id: &str,
        action: &str,
        retry_failed_times: Option<u8>,
    ) -> Result<IdeStatusActionResult, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "IDE 账号不存在".to_string())?;

        let normalized_action = action.trim().to_ascii_lowercase();
        if normalized_action != "daily_checkin" {
            return Err(format!("暂不支持的平台动作: {}", action));
        }

        let platform = account.origin_platform.to_ascii_lowercase();
        if !matches!(platform.as_str(), "codebuddy_cn" | "workbuddy") {
            return Err(format!("{} 暂不支持 daily_checkin", account.origin_platform));
        }

        let retry_limit = retry_failed_times
            .unwrap_or(1)
            .min(MAX_RETRY_FAILED_TIMES) as u32;
        let mut attempts = 0u32;
        let mut last_err: Option<String> = None;

        while attempts <= retry_limit {
            attempts += 1;
            match Self::perform_daily_checkin(&account).await {
                Ok(resp) => {
                    let now = Utc::now();
                    account.last_used = now;
                    account.updated_at = now;
                    account.meta_json = Some(Self::merge_status_action_meta(
                        account.meta_json.as_deref(),
                        &normalized_action,
                        &resp,
                        now.timestamp_millis(),
                    ));
                    db.upsert_ide_account(&account).map_err(|e| e.to_string())?;

                    let message = resp.message.clone().unwrap_or_else(|| {
                        if resp.success {
                            "每日签到成功".to_string()
                        } else {
                            "每日签到未完成".to_string()
                        }
                    });

                    return Ok(IdeStatusActionResult {
                        account_id: account.id.clone(),
                        platform: account.origin_platform.clone(),
                        action: normalized_action,
                        success: resp.success,
                        message,
                        reward: resp.reward,
                        next_checkin_in: resp.next_checkin_in,
                        attempts,
                        retried: attempts > 1,
                        retryable: resp.retryable,
                        executed_at: now.to_rfc3339(),
                    });
                }
                Err(err) => {
                    last_err = Some(err.clone());
                    if attempts > retry_limit {
                        return Err(format!(
                            "执行 daily_checkin 失败 (attempts={}): {}",
                            attempts, err
                        ));
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| "执行平台动作失败".to_string()))
    }

    async fn perform_daily_checkin(account: &IdeAccount) -> Result<DailyCheckinResponse, String> {
        let access_token = account.token.access_token.trim();
        if access_token.is_empty() {
            return Err("账号缺少 access_token，无法执行 daily_checkin".to_string());
        }

        let auth_ctx = Self::extract_auth_context(account.meta_json.as_deref());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
        let url = format!("{}/v2/billing/meter/daily-checkin", CODEBUDDY_CN_API_ENDPOINT);

        let mut req = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&json!({}));

        if let Some(uid) = auth_ctx.uid.as_deref() {
            req = req.header("X-User-Id", uid);
        }
        if let Some(enterprise_id) = auth_ctx.enterprise_id.as_deref() {
            req = req.header("X-Enterprise-Id", enterprise_id);
            req = req.header("X-Tenant-Id", enterprise_id);
        }
        if let Some(domain) = auth_ctx.domain.as_deref() {
            req = req.header("X-Domain", domain);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("请求 daily-checkin 失败: {}", e))?;
        let status_code = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析 daily-checkin 响应失败: {}", e))?;

        if !status_code.is_success() {
            let message = body
                .get("message")
                .or_else(|| body.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(format!(
                "请求 daily-checkin 失败 (http={}): {}",
                status_code.as_u16(),
                message
            ));
        }

        let code = body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        let api_message = body
            .get("message")
            .or_else(|| body.get("msg"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string();

        if code != 0 && code != 200 {
            return Ok(DailyCheckinResponse {
                success: false,
                message: Some(api_message),
                reward: None,
                next_checkin_in: None,
                retryable: false,
            });
        }

        let data = body.get("data").cloned().unwrap_or_else(|| json!({}));
        let success = data
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let message = data
            .get("message")
            .or_else(|| body.get("message"))
            .or_else(|| body.get("msg"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let reward = data.get("reward").cloned();
        let next_checkin_in = data
            .get("nextCheckinIn")
            .or_else(|| data.get("next_checkin_in"))
            .and_then(|v| v.as_i64());

        Ok(DailyCheckinResponse {
            success,
            message,
            reward,
            next_checkin_in,
            retryable: false,
        })
    }

    fn merge_status_action_meta(
        original_meta_json: Option<&str>,
        action: &str,
        resp: &DailyCheckinResponse,
        timestamp_ms: i64,
    ) -> String {
        let mut root = original_meta_json
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .unwrap_or_else(|| json!({}));
        if !root.is_object() {
            root = json!({});
        }

        let action_payload = json!({
            "last_attempt_at": timestamp_ms,
            "success": resp.success,
            "message": resp.message,
            "reward": resp.reward,
            "next_checkin_in": resp.next_checkin_in,
        });

        let object = root.as_object_mut().expect("root must be object");
        let status_actions = object
            .entry("status_actions".to_string())
            .or_insert_with(|| json!({}));
        if !status_actions.is_object() {
            *status_actions = json!({});
        }
        if let Some(map) = status_actions.as_object_mut() {
            map.insert(action.to_string(), action_payload);
        }

        serde_json::to_string(&root).unwrap_or_else(|_| "{}".to_string())
    }

    fn extract_auth_context(meta_json: Option<&str>) -> PlatformAuthContext {
        let Some(meta_raw) = meta_json else {
            return PlatformAuthContext::default();
        };
        let Ok(meta_value) = serde_json::from_str::<Value>(meta_raw) else {
            return PlatformAuthContext::default();
        };

        PlatformAuthContext {
            uid: Self::pick_string_recursive(&meta_value, &["uid", "user_id", "userId"]),
            enterprise_id: Self::pick_string_recursive(
                &meta_value,
                &["enterprise_id", "enterpriseId", "tenant_id", "tenantId"],
            ),
            domain: Self::pick_string_recursive(&meta_value, &["domain"]),
        }
    }

    fn pick_string_recursive(value: &Value, keys: &[&str]) -> Option<String> {
        match value {
            Value::Object(map) => {
                for key in keys {
                    if let Some(found) = map
                        .get(*key)
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    {
                        return Some(found.to_string());
                    }
                }
                for nested in map.values() {
                    if let Some(found) = Self::pick_string_recursive(nested, keys) {
                        return Some(found);
                    }
                }
                None
            }
            Value::Array(items) => items
                .iter()
                .find_map(|item| Self::pick_string_recursive(item, keys)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DailyCheckinResponse, PlatformStatusActionService};
    use serde_json::json;

    #[test]
    fn pick_string_recursive_finds_nested_values() {
        let payload = json!({
            "meta": {
                "account": {
                    "userId": "  user-123  "
                }
            },
            "items": [
                {"enterpriseId": "ent-001"},
                {"domain": "tenant.example.com"}
            ]
        });

        let uid = PlatformStatusActionService::pick_string_recursive(
            &payload,
            &["uid", "user_id", "userId"],
        );
        let enterprise = PlatformStatusActionService::pick_string_recursive(
            &payload,
            &["enterprise_id", "enterpriseId"],
        );
        let domain = PlatformStatusActionService::pick_string_recursive(&payload, &["domain"]);

        assert_eq!(uid.as_deref(), Some("user-123"));
        assert_eq!(enterprise.as_deref(), Some("ent-001"));
        assert_eq!(domain.as_deref(), Some("tenant.example.com"));
    }

    #[test]
    fn extract_auth_context_handles_invalid_meta() {
        let ctx = PlatformStatusActionService::extract_auth_context(Some("not-json"));
        assert!(ctx.uid.is_none());
        assert!(ctx.enterprise_id.is_none());
        assert!(ctx.domain.is_none());
    }

    #[test]
    fn merge_status_action_meta_inserts_payload_and_preserves_existing_fields() {
        let existing = json!({
            "uid": "u-1",
            "profile": {"nickname": "tester"},
            "status_actions": {
                "legacy_action": {"success": true}
            }
        })
        .to_string();
        let resp = DailyCheckinResponse {
            success: true,
            message: Some("ok".to_string()),
            reward: Some(json!({"credit": 2})),
            next_checkin_in: Some(86400),
            retryable: false,
        };

        let merged = PlatformStatusActionService::merge_status_action_meta(
            Some(existing.as_str()),
            "daily_checkin",
            &resp,
            123456789,
        );
        let merged_value = serde_json::from_str::<serde_json::Value>(&merged).unwrap();

        assert_eq!(merged_value["uid"], json!("u-1"));
        assert_eq!(
            merged_value["status_actions"]["legacy_action"]["success"],
            json!(true)
        );
        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["last_attempt_at"],
            json!(123456789)
        );
        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["success"],
            json!(true)
        );
        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["reward"]["credit"],
            json!(2)
        );
        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["next_checkin_in"],
            json!(86400)
        );
    }

    #[test]
    fn merge_status_action_meta_recovers_from_non_object_meta() {
        let resp = DailyCheckinResponse {
            success: false,
            message: Some("already checked".to_string()),
            reward: None,
            next_checkin_in: None,
            retryable: false,
        };
        let merged = PlatformStatusActionService::merge_status_action_meta(
            Some("[1,2,3]"),
            "daily_checkin",
            &resp,
            99,
        );
        let merged_value = serde_json::from_str::<serde_json::Value>(&merged).unwrap();

        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["success"],
            json!(false)
        );
        assert_eq!(
            merged_value["status_actions"]["daily_checkin"]["message"],
            json!("already checked")
        );
    }
}
