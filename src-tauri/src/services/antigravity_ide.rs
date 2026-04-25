use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use crate::services::account_health::AccountHealthService;
use crate::services::ide_injector::read_antigravity_secret_storage_value;
use chrono::Utc;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{Map, Value};

const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const ANTIGRAVITY_CLIENT_SECRET_ENV: &str = "AIS_ANTIGRAVITY_CLIENT_SECRET";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

#[derive(Debug, Deserialize)]
struct GoogleTokenRefreshResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    token_type: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfoResponse {
    id: Option<String>,
    email: Option<String>,
    name: Option<String>,
}

pub struct AntigravityIdeService;

impl AntigravityIdeService {
    pub async fn refresh_all_accounts(db: &Database) -> Result<usize, String> {
        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case("antigravity"))
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
        match Self::refresh_account_inner(db, account_id).await {
            Ok(account) => {
                AccountHealthService::try_clear_invalid_grant(db, &account);
                AccountHealthService::clear_quota_error(db, &account.id);
                Ok(account)
            }
            Err(err) => {
                if AccountHealthService::looks_like_invalid_grant(&err) {
                    AccountHealthService::mark_invalid_grant(db, account_id, &err);
                } else {
                    AccountHealthService::record_quota_error(db, account_id, None, &err);
                }
                Err(err)
            }
        }
    }

    async fn refresh_account_inner(
        db: &Database,
        account_id: &str,
    ) -> Result<IdeAccount, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Antigravity 账号不存在".to_string())?;

        if !account.origin_platform.eq_ignore_ascii_case("antigravity") {
            return Err("当前账号不是 Antigravity 类型".to_string());
        }

        let mut access_token = account.token.access_token.trim().to_string();
        let refresh_token = account.token.refresh_token.trim().to_string();
        let now_ms = Utc::now().timestamp_millis();
        let expires_at_ms =
            account.token.updated_at.timestamp_millis() + (account.token.expires_in as i64 * 1000);

        let mut userinfo = if access_token.is_empty() || access_token == "requires_refresh" {
            None
        } else {
            Self::fetch_google_userinfo(&access_token).await
        };

        if userinfo.is_none() || now_ms >= expires_at_ms.saturating_sub(60_000) {
            let refreshed = Self::refresh_access_token(&refresh_token).await?;
            let refreshed_access_token = refreshed
                .access_token
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "Antigravity 刷新响应缺少 access_token".to_string())?;
            access_token = refreshed_access_token;
            account.token.access_token = access_token.clone();
            if let Some(expires_in) = refreshed.expires_in {
                account.token.expires_in = expires_in;
            }
            if let Some(token_type) = refreshed
                .token_type
                .filter(|value| !value.trim().is_empty())
            {
                account.token.token_type = token_type;
            }
            account.token.updated_at = Utc::now();
            userinfo = Self::fetch_google_userinfo(&access_token).await;
        }

        if let Some(info) = userinfo {
            if let Some(email) = info.email.filter(|value| !value.trim().is_empty()) {
                account.email = email;
            }
            if let Some(name) = info.name.filter(|value| !value.trim().is_empty()) {
                account.label = Some(name.clone());
            }

            let mut meta = parse_meta_object(account.meta_json.as_deref());
            meta.insert("auth_mode".to_string(), Value::String("oauth".to_string()));
            meta.insert(
                "oauth_provider".to_string(),
                Value::String("antigravity".to_string()),
            );
            if let Some(user_id) = info.id.filter(|value| !value.trim().is_empty()) {
                meta.insert("user_id".to_string(), Value::String(user_id));
            }
            enrich_local_antigravity_meta(&mut meta);
            meta.insert(
                "synced_at".to_string(),
                Value::String(Utc::now().to_rfc3339()),
            );
            account.meta_json = Some(Value::Object(meta).to_string());
        }

        account.status = AccountStatus::Active;
        account.disabled_reason = None;
        account.updated_at = Utc::now();
        account.last_used = Utc::now();

        db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
        Ok(account)
    }

    async fn refresh_access_token(
        refresh_token: &str,
    ) -> Result<GoogleTokenRefreshResponse, String> {
        if refresh_token.is_empty() || refresh_token == "missing" {
            return Err("Antigravity refresh_token 不存在，无法刷新 access_token".to_string());
        }

        let client_secret = std::env::var(ANTIGRAVITY_CLIENT_SECRET_ENV).map_err(|_| {
            format!(
                "未配置环境变量 {}，无法刷新 Antigravity access_token",
                ANTIGRAVITY_CLIENT_SECRET_ENV
            )
        })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client
            .post(GOOGLE_TOKEN_ENDPOINT)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[
                ("client_id", ANTIGRAVITY_CLIENT_ID),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("刷新 Antigravity access_token 请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "刷新 Antigravity access_token 失败: status={}, body={}",
                status, body
            ));
        }

        let payload = response
            .json::<GoogleTokenRefreshResponse>()
            .await
            .map_err(|e| format!("解析 Antigravity access_token 刷新响应失败: {}", e))?;
        if payload.access_token.is_none() {
            return Err(format!(
                "刷新 Antigravity access_token 响应异常: error={:?}, desc={:?}",
                payload.error, payload.error_description
            ));
        }
        Ok(payload)
    }

    async fn fetch_google_userinfo(access_token: &str) -> Option<GoogleUserInfoResponse> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .ok()?;
        let response = client
            .get(GOOGLE_USERINFO_ENDPOINT)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.json::<GoogleUserInfoResponse>().await.ok()
    }
}

fn parse_meta_object(raw: Option<&str>) -> Map<String, Value> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn enrich_local_antigravity_meta(meta: &mut Map<String, Value>) {
    let Some(data_root) = antigravity_app_data_root() else {
        return;
    };

    let auth_status_path = data_root
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if auth_status_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open(&auth_status_path) {
            if let Some(raw) = conn
                .query_row(
                    "SELECT value FROM ItemTable WHERE key = ?1 LIMIT 1",
                    ["antigravityAuthStatus"],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .filter(|value| !value.trim().is_empty())
            {
                if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
                    meta.insert("antigravity_auth_status_raw".to_string(), parsed);
                }
            }
        }
    }

    let storage_path = data_root
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    if storage_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&storage_path) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
                if let Some(project_id) = parsed
                    .get("antigravityUnifiedStateSync.oauthToken")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                {
                    meta.entry("antigravity_oauth_topic_raw".to_string())
                        .or_insert_with(|| Value::String(project_id.to_string()));
                }
                meta.insert("antigravity_storage_raw".to_string(), parsed);
            }
        }
    }

    if let Ok(Some(raw)) = read_antigravity_secret_storage_value(
        "jlcodes.antigravity-cockpit",
        "antigravity.autoTrigger.credential",
        Some(data_root.to_string_lossy().as_ref()),
    ) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            if let Some(project_id) = parsed
                .get("projectId")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            {
                meta.insert(
                    "antigravity_project_id".to_string(),
                    Value::String(project_id.to_string()),
                );
            }
            meta.insert("antigravity_secret_credential_raw".to_string(), parsed);
        }
    }

    if let Ok(Some(raw)) = read_antigravity_secret_storage_value(
        "jlcodes.antigravity-cockpit",
        "antigravity.autoTrigger.credentials",
        Some(data_root.to_string_lossy().as_ref()),
    ) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            meta.insert("antigravity_secret_credentials_raw".to_string(), parsed);
        }
    }
}

fn antigravity_app_data_root() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return std::env::var("APPDATA")
            .ok()
            .map(std::path::PathBuf::from)
            .map(|path| path.join("Antigravity"));
    }

    #[cfg(target_os = "macos")]
    {
        return dirs::home_dir().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Antigravity")
        });
    }

    #[cfg(target_os = "linux")]
    {
        return dirs::home_dir().map(|home| home.join(".config").join("Antigravity"));
    }

    #[allow(unreachable_code)]
    None
}
