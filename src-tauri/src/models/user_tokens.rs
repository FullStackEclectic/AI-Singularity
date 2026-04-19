use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserToken {
    pub id: String,
    pub token: String,
    pub username: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub expires_type: String,
    pub expires_at: Option<i64>,
    pub max_ips: i64,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
    pub total_requests: i64,
    pub total_tokens_used: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenScope {
    #[serde(default = "default_scope")]
    pub scope: String,
    pub desc: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub single_account: Option<String>,
}

fn default_scope() -> String {
    "global".to_string()
}

impl UserToken {
    pub fn parse_scope(&self) -> TokenScope {
        if let Some(desc) = &self.description {
            if desc.starts_with('{') && desc.contains("\"scope\"") {
                if let Ok(scope) = serde_json::from_str::<TokenScope>(desc) {
                    return scope;
                }
            }
        }
        TokenScope {
            scope: "global".to_string(),
            desc: self.description.clone(),
            channels: vec![],
            tags: vec![],
            single_account: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserTokenReq {
    pub username: String,
    pub description: Option<String>,
    pub expires_type: String,
    pub expires_at: Option<i64>,
    pub max_ips: i64,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserTokenReq {
    pub id: String,
    pub username: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub expires_type: Option<String>,
    pub expires_at: Option<i64>,
    pub max_ips: Option<i64>,
    pub curfew_start: Option<String>,
    pub curfew_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokenSummary {
    pub total_tokens: i64,
    pub active_tokens: i64,
    pub total_users: i64,
    pub today_requests: i64,
}
