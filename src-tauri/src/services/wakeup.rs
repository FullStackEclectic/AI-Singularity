mod batch;
mod execution;
mod scheduler;
mod storage;

use self::execution::{
    normalize_client_version_mode,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const WAKEUP_STATE_FILE: &str = "wakeup_state.json";
const WAKEUP_HISTORY_FILE: &str = "wakeup_history.json";
const MAX_HISTORY_ITEMS: usize = 200;
const SCHEDULER_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupTask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub account_id: String,
    #[serde(default = "default_trigger_mode")]
    pub trigger_mode: String,
    #[serde(default = "default_reset_window")]
    pub reset_window: String,
    #[serde(default = "default_window_day_policy")]
    pub window_day_policy: String,
    #[serde(default = "default_window_fallback_policy")]
    pub window_fallback_policy: String,
    #[serde(default = "default_client_version_mode")]
    pub client_version_mode: String,
    #[serde(default = "default_client_version_fallback_mode")]
    pub client_version_fallback_mode: String,
    #[serde(default)]
    pub command_template: String,
    pub model: String,
    pub prompt: String,
    pub cron: String,
    pub notes: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retry_failed_times: u8,
    #[serde(default)]
    pub pause_after_failures: u8,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_category: Option<String>,
    #[serde(default)]
    pub last_message: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WakeupState {
    pub enabled: bool,
    pub tasks: Vec<WakeupTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupHistoryItem {
    pub id: String,
    #[serde(default)]
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub task_name: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    #[serde(default = "default_wakeup_category")]
    pub category: String,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchItem {
    pub account_id: String,
    pub email: String,
    pub status: String,
    pub category: String,
    pub attempts: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupCategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchResult {
    pub executed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub retried_count: usize,
    pub canceled: bool,
    pub category_counts: Vec<WakeupCategoryCount>,
    pub items: Vec<WakeupVerificationBatchItem>,
}

pub struct WakeupService;
static WAKEUP_SCHEDULER_STARTED: OnceLock<()> = OnceLock::new();

fn default_timeout_seconds() -> u64 {
    120
}

fn default_trigger_mode() -> String {
    "cron".to_string()
}

fn default_reset_window() -> String {
    "primary_window".to_string()
}

fn default_window_day_policy() -> String {
    "all_days".to_string()
}

fn default_window_fallback_policy() -> String {
    "none".to_string()
}

fn default_wakeup_category() -> String {
    "unknown".to_string()
}

fn default_client_version_mode() -> String {
    "auto".to_string()
}

fn default_client_version_fallback_mode() -> String {
    "auto".to_string()
}

impl WakeupService {
}
