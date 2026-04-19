use serde::{Deserialize, Serialize};

pub(super) const TRAY_ID: &str = "main_tray";
pub(super) const TRAY_SCOPE_FILE: &str = "tray_scope.json";
pub(super) const QUICK_SWITCH_MENU_PREFIX: &str = "quick_switch_account_";
pub(super) const QUICK_SWITCH_MAX_PER_PLATFORM: usize = 8;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrayScopeState {
    #[serde(default)]
    pub platforms: Vec<String>,
    pub updated_at: Option<String>,
}
