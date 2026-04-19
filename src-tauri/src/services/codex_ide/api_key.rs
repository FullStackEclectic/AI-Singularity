use super::helpers::{normalize_api_base_url, normalize_api_key, parse_meta_json_object};
use super::CodexIdeService;
use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use chrono::Utc;
use serde_json::Value;

impl CodexIdeService {
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
        meta.insert(
            "plan_type".to_string(),
            Value::String("API_KEY".to_string()),
        );

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
}
