use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use chrono::Utc;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;

const GEMINI_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const GEMINI_CLIENT_SECRET_ENV: &str = "AIS_GEMINI_CLIENT_SECRET";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const CODE_ASSIST_LOAD_ENDPOINT: &str =
    "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
const CODE_ASSIST_QUOTA_ENDPOINT: &str =
    "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota";

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

#[derive(Debug)]
struct LoadCodeAssistStatus {
    project_id: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct GeminiCloudProject {
    pub project_id: String,
    pub project_name: Option<String>,
}

pub struct GeminiIdeService;

impl GeminiIdeService {
    pub async fn refresh_all_accounts(db: &Database) -> Result<usize, String> {
        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|item| item.origin_platform.eq_ignore_ascii_case("gemini"))
            .collect::<Vec<_>>();

        let mut success = 0usize;
        for account in accounts {
            if Self::refresh_account(db, &account.id).await.is_ok() {
                success += 1;
            }
        }
        Ok(success)
    }

    pub fn set_project_id(
        db: &Database,
        account_id: &str,
        project_id: Option<&str>,
    ) -> Result<IdeAccount, String> {
        db.update_ide_account_project_id(account_id, project_id)
            .map_err(|e| e.to_string())?;
        db.get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Gemini 账号不存在".to_string())
    }

    pub async fn list_cloud_projects(
        db: &Database,
        account_id: &str,
    ) -> Result<Vec<GeminiCloudProject>, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Gemini 账号不存在".to_string())?;
        if !account.origin_platform.eq_ignore_ascii_case("gemini") {
            return Err("当前账号不是 Gemini 类型".to_string());
        }

        let mut access_token = account.token.access_token.clone();
        let refresh_token = account.token.refresh_token.clone();
        let now_ms = Utc::now().timestamp_millis();
        let expires_at_ms =
            account.token.updated_at.timestamp_millis() + (account.token.expires_in as i64 * 1000);
        if access_token.trim().is_empty() {
            return Err("Gemini 账号缺少 access_token".to_string());
        }

        let mut projects = Self::fetch_project_candidates(&access_token).await;
        if projects.is_err() && now_ms >= expires_at_ms.saturating_sub(60_000) {
            if let Ok(refreshed) = Self::refresh_access_token(&refresh_token).await {
                if let Some(token) = refreshed.access_token {
                    access_token = token;
                    account.token.access_token = access_token.clone();
                    if let Some(expires_in) = refreshed.expires_in {
                        account.token.expires_in = expires_in;
                    }
                    if let Some(token_type) = refreshed.token_type {
                        account.token.token_type = token_type;
                    }
                    account.token.updated_at = Utc::now();
                    db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
                    projects = Self::fetch_project_candidates(&access_token).await;
                }
            }
        }

        projects
    }

    pub async fn refresh_account(db: &Database, account_id: &str) -> Result<IdeAccount, String> {
        let mut account = db
            .get_all_ide_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.id == account_id)
            .ok_or_else(|| "Gemini 账号不存在".to_string())?;

        if !account.origin_platform.eq_ignore_ascii_case("gemini") {
            return Err("当前账号不是 Gemini 类型".to_string());
        }

        let mut access_token = account.token.access_token.clone();
        let refresh_token = account.token.refresh_token.clone();
        let now_ms = Utc::now().timestamp_millis();
        let expires_at_ms =
            account.token.updated_at.timestamp_millis() + (account.token.expires_in as i64 * 1000);

        if access_token.trim().is_empty() {
            return Err("Gemini 账号缺少 access_token".to_string());
        }

        let mut load_status = Self::load_code_assist_status(&access_token).await;
        if load_status.is_err() && now_ms >= expires_at_ms.saturating_sub(60_000) {
            if let Ok(refreshed) = Self::refresh_access_token(&refresh_token).await {
                if let Some(token) = refreshed.access_token {
                    access_token = token;
                    account.token.access_token = access_token.clone();
                    if let Some(expires_in) = refreshed.expires_in {
                        account.token.expires_in = expires_in;
                    }
                    if let Some(token_type) = refreshed.token_type {
                        account.token.token_type = token_type;
                    }
                    account.token.updated_at = Utc::now();
                    load_status = Self::load_code_assist_status(&access_token).await;
                }
            }
        }
        let load_status = load_status?;

        let project_id = account
            .project_id
            .clone()
            .or(load_status.project_id.clone());
        let quota = Self::retrieve_user_quota(&access_token, project_id.as_deref()).await?;

        if let Some(userinfo) = Self::fetch_google_userinfo(&access_token).await {
            if let Some(email) = userinfo.email.filter(|s| !s.trim().is_empty()) {
                account.email = email;
            }
            if account.disabled_reason.is_none() {
                account.disabled_reason = userinfo.name;
            }
            if account.email.trim().is_empty() {
                if let Some(id) = userinfo.id {
                    account.email = format!("{}@gmail.com", id);
                }
            }
        }

        account.status = AccountStatus::Active;
        account.disabled_reason = None;
        account.quota_json = Some(
            serde_json::json!({
                "project_id": project_id,
                "quota": quota,
                "synced_at": Utc::now().to_rfc3339(),
            })
            .to_string(),
        );
        account.updated_at = Utc::now();
        account.last_used = Utc::now();

        db.upsert_ide_account(&account).map_err(|e| e.to_string())?;
        Ok(account)
    }

    async fn refresh_access_token(
        refresh_token: &str,
    ) -> Result<GoogleTokenRefreshResponse, String> {
        if refresh_token.trim().is_empty() || refresh_token == "missing" {
            return Err("Gemini refresh_token 不存在，无法刷新 access_token".to_string());
        }
        let client_secret = std::env::var(GEMINI_CLIENT_SECRET_ENV).map_err(|_| {
            format!(
                "未配置环境变量 {}，无法刷新 Gemini access_token",
                GEMINI_CLIENT_SECRET_ENV
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
                ("client_id", GEMINI_CLIENT_ID),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("刷新 Gemini access_token 请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "刷新 Gemini access_token 失败: status={}, body={}",
                status, body
            ));
        }

        let payload = response
            .json::<GoogleTokenRefreshResponse>()
            .await
            .map_err(|e| format!("解析 Gemini access_token 刷新响应失败: {}", e))?;
        if payload.access_token.is_none() {
            return Err(format!(
                "刷新 Gemini access_token 响应异常: error={:?}, desc={:?}",
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

    async fn load_code_assist_status(access_token: &str) -> Result<LoadCodeAssistStatus, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client
            .post(CODE_ASSIST_LOAD_ENDPOINT)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({
                "metadata": {
                    "ideType": "GEMINI_CLI",
                    "pluginType": "GEMINI"
                }
            }))
            .send()
            .await
            .map_err(|e| format!("请求 Gemini loadCodeAssist 失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "请求 Gemini loadCodeAssist 失败: status={}, body={}",
                status, body
            ));
        }

        let value = response
            .json::<Value>()
            .await
            .map_err(|e| format!("解析 Gemini loadCodeAssist 响应失败: {}", e))?;

        let project_id = value
            .get("cloudaicompanionProject")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                value
                    .get("cloudaicompanionProject")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                value
                    .get("cloudaicompanionProject")
                    .and_then(|v| v.get("projectId"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        Ok(LoadCodeAssistStatus { project_id })
    }

    async fn retrieve_user_quota(
        access_token: &str,
        project_id: Option<&str>,
    ) -> Result<Value, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let body = if let Some(project_id) = project_id.filter(|s| !s.trim().is_empty()) {
            serde_json::json!({ "project": project_id })
        } else {
            serde_json::json!({})
        };

        let response = client
            .post(CODE_ASSIST_QUOTA_ENDPOINT)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("请求 Gemini retrieveUserQuota 失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "请求 Gemini retrieveUserQuota 失败: status={}, body={}",
                status, body
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("解析 Gemini retrieveUserQuota 响应失败: {}", e))
    }

    async fn fetch_project_candidates(
        access_token: &str,
    ) -> Result<Vec<GeminiCloudProject>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client
            .get("https://cloudresourcemanager.googleapis.com/v1/projects")
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("请求 Google projects 列表失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<empty-body>".to_string());
            return Err(format!(
                "请求 Google projects 列表失败: status={}, body={}",
                status, body
            ));
        }

        let value = response
            .json::<Value>()
            .await
            .map_err(|e| format!("解析 Google projects 列表失败: {}", e))?;

        let Some(projects) = value.get("projects").and_then(|v| v.as_array()) else {
            return Ok(vec![]);
        };

        let mut result = Vec::new();
        for project in projects {
            let lifecycle = project
                .get("lifecycleState")
                .and_then(|v| v.as_str())
                .unwrap_or("ACTIVE");
            if !lifecycle.eq_ignore_ascii_case("ACTIVE") {
                continue;
            }
            let project_id = project
                .get("projectId")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let Some(project_id) = project_id else {
                continue;
            };
            let project_name = project
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            result.push(GeminiCloudProject {
                project_id: project_id.to_string(),
                project_name,
            });
        }

        result.sort_by(|a, b| a.project_id.cmp(&b.project_id));
        Ok(result)
    }
}
