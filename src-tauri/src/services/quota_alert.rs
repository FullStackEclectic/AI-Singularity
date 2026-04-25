use crate::db::Database;
use crate::models::IdeAccount;
use crate::services::event_bus::EventBus;
use crate::services::quota_parser::QuotaSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter};

static QUOTA_ALERT_LAST_SENT: LazyLock<Mutex<HashMap<String, i64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const DEFAULT_THRESHOLD: i32 = 10;
const DEFAULT_COOLDOWN_SECONDS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaAlertSettings {
    pub enabled: bool,
    pub threshold: i32,
    pub cooldown_seconds: i64,
}

impl Default for QuotaAlertSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: DEFAULT_THRESHOLD,
            cooldown_seconds: DEFAULT_COOLDOWN_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaAlertPayload {
    pub account_id: String,
    pub email: String,
    pub origin_platform: String,
    pub threshold: i32,
    pub lowest_percentage: i32,
    pub low_models: Vec<String>,
    pub triggered_at: i64,
}

pub struct QuotaAlertService;

impl QuotaAlertService {
    pub fn normalize_threshold(raw: i32) -> i32 {
        raw.clamp(0, 100)
    }

    pub fn normalize_cooldown(raw: i64) -> i64 {
        raw.clamp(30, 86_400)
    }

    pub fn load_settings(db: &Database) -> QuotaAlertSettings {
        let mut s = QuotaAlertSettings::default();
        let map = db.get_all_account_settings().unwrap_or_default();
        if let Some(v) = map.get("quota_alert_enabled") {
            if let Ok(b) = v.parse::<bool>() {
                s.enabled = b;
            }
        }
        if let Some(v) = map.get("quota_alert_threshold") {
            if let Ok(n) = v.parse::<i32>() {
                s.threshold = Self::normalize_threshold(n);
            }
        }
        if let Some(v) = map.get("quota_alert_cooldown_seconds") {
            if let Ok(n) = v.parse::<i64>() {
                s.cooldown_seconds = Self::normalize_cooldown(n);
            }
        }
        s
    }

    pub fn save_settings(db: &Database, settings: &QuotaAlertSettings) -> Result<(), String> {
        let kvs = vec![
            ("quota_alert_enabled".to_string(), settings.enabled.to_string()),
            (
                "quota_alert_threshold".to_string(),
                Self::normalize_threshold(settings.threshold).to_string(),
            ),
            (
                "quota_alert_cooldown_seconds".to_string(),
                Self::normalize_cooldown(settings.cooldown_seconds).to_string(),
            ),
        ];
        db.set_account_settings_batch(&kvs)
            .map_err(|e| e.to_string())
    }

    fn build_cooldown_key(account_id: &str, threshold: i32) -> String {
        format!("{}:{}", account_id, threshold)
    }

    fn should_emit(key: &str, now: i64, cooldown: i64) -> bool {
        let mut state = match QUOTA_ALERT_LAST_SENT.lock() {
            Ok(g) => g,
            Err(_) => return true,
        };
        if let Some(last) = state.get(key) {
            if now - *last < cooldown {
                return false;
            }
        }
        state.insert(key.to_string(), now);
        true
    }

    fn clear_cooldown(account_id: &str, threshold: i32) {
        if let Ok(mut state) = QUOTA_ALERT_LAST_SENT.lock() {
            state.remove(&Self::build_cooldown_key(account_id, threshold));
        }
    }

    pub fn run_if_needed(db: &Database, app: Option<&AppHandle>) -> Vec<QuotaAlertPayload> {
        let settings = Self::load_settings(db);
        if !settings.enabled {
            return Vec::new();
        }
        let threshold = Self::normalize_threshold(settings.threshold);
        let cooldown = Self::normalize_cooldown(settings.cooldown_seconds);
        let accounts = db.get_all_ide_accounts().unwrap_or_default();
        let now = chrono::Utc::now().timestamp();

        let mut payloads = Vec::new();
        for account in &accounts {
            if let Some(payload) =
                Self::evaluate_account(account, threshold, cooldown, now)
            {
                if let Some(app_handle) = app {
                    let _ = app_handle.emit("accounts:quota-alert", &payload);
                    EventBus::emit_data_changed(
                        app_handle,
                        "ide_accounts",
                        "quota_alert",
                        "quota_alert.run",
                    );
                }
                tracing::warn!(
                    "[QuotaAlert] {} ({}) 触发配额告警: lowest={}% threshold={}%",
                    payload.email,
                    payload.account_id,
                    payload.lowest_percentage,
                    payload.threshold
                );
                payloads.push(payload);
            }
        }
        payloads
    }

    fn evaluate_account(
        account: &IdeAccount,
        threshold: i32,
        cooldown: i64,
        now: i64,
    ) -> Option<QuotaAlertPayload> {
        if matches!(account.status, crate::models::AccountStatus::Forbidden) {
            Self::clear_cooldown(&account.id, threshold);
            return None;
        }
        let summary = QuotaSummary::parse(account)?;

        let low_models: Vec<(String, i32)> = if summary.is_forbidden {
            vec![("all".to_string(), 0)]
        } else {
            summary
                .models
                .iter()
                .filter(|m| m.percentage <= threshold)
                .map(|m| (m.name.clone(), m.percentage))
                .collect()
        };

        if low_models.is_empty() {
            Self::clear_cooldown(&account.id, threshold);
            return None;
        }

        let key = Self::build_cooldown_key(&account.id, threshold);
        if !Self::should_emit(&key, now, cooldown) {
            return None;
        }

        let lowest = low_models.iter().map(|(_, p)| *p).min().unwrap_or(0);
        Some(QuotaAlertPayload {
            account_id: account.id.clone(),
            email: account.email.clone(),
            origin_platform: account.origin_platform.clone(),
            threshold,
            lowest_percentage: lowest,
            low_models: low_models.into_iter().map(|(n, _)| n).collect(),
            triggered_at: now,
        })
    }
}
