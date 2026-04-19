use serde::{Deserialize, Serialize};

pub(super) const CONFIG_FILE: &str = "update_settings.json";
pub(super) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub auto_install: bool,
    #[serde(default)]
    pub skip_version: Option<String>,
    #[serde(default)]
    pub disable_reminders: bool,
    #[serde(default = "default_silent_reminder_strategy")]
    pub silent_reminder_strategy: String,
    #[serde(default)]
    pub last_reminded_at: Option<String>,
    #[serde(default)]
    pub last_reminded_version: Option<String>,
    pub last_check_at: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_install: false,
            skip_version: None,
            disable_reminders: false,
            silent_reminder_strategy: default_silent_reminder_strategy(),
            last_reminded_at: None,
            last_reminded_version: None,
            last_check_at: None,
        }
    }
}

fn default_silent_reminder_strategy() -> String {
    "immediate".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReminderDecision {
    pub should_notify: bool,
    pub reason: String,
    pub settings: UpdateSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRuntimeInfo {
    pub current_version: String,
    pub platform: String,
    pub updater_endpoints: Vec<String>,
    pub updater_pubkey_configured: bool,
    pub can_auto_install: bool,
    pub linux_install_kind: Option<String>,
    pub linux_manual_hint: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxReleaseAssetInfo {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub preferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxReleaseInfo {
    pub version: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<LinuxReleaseAssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxInstallResult {
    pub downloaded_path: String,
    pub action: String,
    pub message: String,
}
