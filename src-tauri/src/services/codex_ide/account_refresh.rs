use super::helpers::{
    decode_jwt_claim, extract_chatgpt_account_id_from_access_token,
    extract_chatgpt_organization_id_from_access_token, get_meta_string, is_token_expired,
    normalize_string, parse_meta_json_object, should_force_refresh_token,
};
use super::CodexIdeService;
use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use chrono::Utc;
use serde_json::Value;

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
}
