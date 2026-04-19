use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DeviceProfile {
    pub machine_id: String,
    pub mac_machine_id: String,
    pub dev_device_id: String,
    pub sqm_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Expired,
    Forbidden,
    RateLimited,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeAccount {
    pub id: String,
    pub email: String,
    pub origin_platform: String,
    pub token: OAuthToken,
    pub status: AccountStatus,
    pub disabled_reason: Option<String>,
    pub is_proxy_disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub device_profile: Option<DeviceProfile>,
    pub quota_json: Option<String>,
    pub project_id: Option<String>,
    pub meta_json: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}
