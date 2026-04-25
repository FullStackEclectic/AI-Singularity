use crate::db::{AccountSwitchHistoryItem, Database};
use crate::models::IdeAccount;
use crate::services::account_health::AccountHealthService;
use crate::services::event_bus::EventBus;
use crate::services::provider_current::ProviderCurrentService;
use crate::services::quota_parser::QuotaSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use uuid::Uuid;

static AUTO_SWITCH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

const AUTO_SWITCH_SCOPE_ANY_GROUP: &str = "any_group";
const AUTO_SWITCH_SCOPE_SELECTED_GROUPS: &str = "selected_groups";
const AUTO_SWITCH_ACCOUNT_SCOPE_ALL: &str = "all_accounts";
const AUTO_SWITCH_ACCOUNT_SCOPE_SELECTED: &str = "selected_accounts";

const RULE_CURRENT_DISABLED: &str = "current_disabled";
const RULE_CURRENT_QUOTA_FORBIDDEN: &str = "current_quota_forbidden";
const RULE_GROUP_BELOW_THRESHOLD: &str = "group_below_threshold";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchSettings {
    pub enabled: bool,
    pub threshold: i32,
    pub scope_mode: String,
    pub selected_group_ids: Vec<String>,
    pub account_scope_mode: String,
    pub selected_account_ids: Vec<String>,
    pub hard_switch_enabled: bool,
}

impl Default for AutoSwitchSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 20,
            scope_mode: AUTO_SWITCH_SCOPE_ANY_GROUP.to_string(),
            selected_group_ids: Vec::new(),
            account_scope_mode: AUTO_SWITCH_ACCOUNT_SCOPE_ALL.to_string(),
            selected_account_ids: Vec::new(),
            hard_switch_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchGroupDefinition {
    pub id: String,
    pub name: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone)]
struct AutoSwitchTriggerContext {
    rule: &'static str,
    threshold: i32,
    scope_mode: String,
    hit_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchOutcome {
    pub triggered: bool,
    pub from_account_id: Option<String>,
    pub to_account_id: Option<String>,
    pub rule: Option<String>,
    pub reason: Option<String>,
}

pub struct AutoSwitchService;

impl AutoSwitchService {
    pub fn default_groups() -> Vec<AutoSwitchGroupDefinition> {
        vec![
            AutoSwitchGroupDefinition {
                id: "claude".to_string(),
                name: "Claude".to_string(),
                models: vec![
                    "claude-opus-4-6".to_string(),
                    "claude-opus-4-6-thinking".to_string(),
                    "claude-sonnet-4-6".to_string(),
                    "claude-sonnet-4-6-thinking".to_string(),
                    "claude-sonnet-4-5".to_string(),
                ],
            },
            AutoSwitchGroupDefinition {
                id: "gemini_pro".to_string(),
                name: "Gemini Pro".to_string(),
                models: vec![
                    "gemini-3-pro-high".to_string(),
                    "gemini-3-pro-low".to_string(),
                    "gemini-3.1-pro-high".to_string(),
                ],
            },
            AutoSwitchGroupDefinition {
                id: "gemini_flash".to_string(),
                name: "Gemini Flash".to_string(),
                models: vec![
                    "gemini-3-flash".to_string(),
                    "gemini-3.1-flash".to_string(),
                    "gemini-3-flash-lite".to_string(),
                ],
            },
            AutoSwitchGroupDefinition {
                id: "codex_hourly".to_string(),
                name: "Codex Hourly".to_string(),
                models: vec!["hourly".to_string()],
            },
            AutoSwitchGroupDefinition {
                id: "codex_weekly".to_string(),
                name: "Codex Weekly".to_string(),
                models: vec!["weekly".to_string()],
            },
        ]
    }

    pub fn normalize_threshold(raw: i32) -> i32 {
        raw.clamp(0, 100)
    }

    fn normalize_scope_mode(raw: &str) -> String {
        let v = raw.trim().to_lowercase();
        if v == AUTO_SWITCH_SCOPE_SELECTED_GROUPS {
            AUTO_SWITCH_SCOPE_SELECTED_GROUPS.to_string()
        } else {
            AUTO_SWITCH_SCOPE_ANY_GROUP.to_string()
        }
    }

    fn normalize_account_scope_mode(raw: &str) -> String {
        let v = raw.trim().to_lowercase();
        if v == AUTO_SWITCH_ACCOUNT_SCOPE_SELECTED {
            AUTO_SWITCH_ACCOUNT_SCOPE_SELECTED.to_string()
        } else {
            AUTO_SWITCH_ACCOUNT_SCOPE_ALL.to_string()
        }
    }

    pub fn load_settings(db: &Database) -> AutoSwitchSettings {
        let mut s = AutoSwitchSettings::default();
        let map = db.get_all_account_settings().unwrap_or_default();
        if let Some(v) = map.get("auto_switch_enabled") {
            if let Ok(b) = v.parse::<bool>() {
                s.enabled = b;
            }
        }
        if let Some(v) = map.get("auto_switch_threshold") {
            if let Ok(n) = v.parse::<i32>() {
                s.threshold = Self::normalize_threshold(n);
            }
        }
        if let Some(v) = map.get("auto_switch_scope_mode") {
            s.scope_mode = Self::normalize_scope_mode(v);
        }
        if let Some(v) = map.get("auto_switch_selected_group_ids") {
            if let Ok(ids) = serde_json::from_str::<Vec<String>>(v) {
                s.selected_group_ids = ids;
            }
        }
        if let Some(v) = map.get("auto_switch_account_scope_mode") {
            s.account_scope_mode = Self::normalize_account_scope_mode(v);
        }
        if let Some(v) = map.get("auto_switch_selected_account_ids") {
            if let Ok(ids) = serde_json::from_str::<Vec<String>>(v) {
                s.selected_account_ids = ids;
            }
        }
        if let Some(v) = map.get("auto_switch_hard_switch_enabled") {
            if let Ok(b) = v.parse::<bool>() {
                s.hard_switch_enabled = b;
            }
        }
        s
    }

    pub fn save_settings(db: &Database, settings: &AutoSwitchSettings) -> Result<(), String> {
        let kvs = vec![
            ("auto_switch_enabled".to_string(), settings.enabled.to_string()),
            (
                "auto_switch_threshold".to_string(),
                Self::normalize_threshold(settings.threshold).to_string(),
            ),
            (
                "auto_switch_scope_mode".to_string(),
                Self::normalize_scope_mode(&settings.scope_mode),
            ),
            (
                "auto_switch_selected_group_ids".to_string(),
                serde_json::to_string(&settings.selected_group_ids).unwrap_or_else(|_| "[]".into()),
            ),
            (
                "auto_switch_account_scope_mode".to_string(),
                Self::normalize_account_scope_mode(&settings.account_scope_mode),
            ),
            (
                "auto_switch_selected_account_ids".to_string(),
                serde_json::to_string(&settings.selected_account_ids).unwrap_or_else(|_| "[]".into()),
            ),
            (
                "auto_switch_hard_switch_enabled".to_string(),
                settings.hard_switch_enabled.to_string(),
            ),
        ];
        db.set_account_settings_batch(&kvs).map_err(|e| e.to_string())
    }

    fn normalize_for_match(value: &str) -> String {
        value.trim().to_lowercase()
    }

    fn model_matches_group(model_name: &str, group_model_id: &str) -> bool {
        let left = Self::normalize_for_match(model_name);
        let right = Self::normalize_for_match(group_model_id);
        if left.is_empty() || right.is_empty() {
            return false;
        }
        if left == right {
            return true;
        }
        left.starts_with(&(right.clone() + "-")) || right.starts_with(&(left + "-"))
    }

    fn account_group_average(account: &IdeAccount, group: &AutoSwitchGroupDefinition) -> Option<i32> {
        let summary = QuotaSummary::parse(account)?;
        let matching: Vec<i32> = summary
            .models
            .iter()
            .filter(|m| {
                group
                    .models
                    .iter()
                    .any(|gm| Self::model_matches_group(&m.name, gm))
            })
            .map(|m| m.percentage)
            .collect();
        if matching.is_empty() {
            return None;
        }
        let sum: i32 = matching.iter().sum();
        Some(sum / matching.len() as i32)
    }

    fn evaluate_trigger(
        account: &IdeAccount,
        threshold: i32,
        scope_mode: &str,
        monitored_groups: &[AutoSwitchGroupDefinition],
    ) -> Option<AutoSwitchTriggerContext> {
        if AccountHealthService::is_invalid_grant_disabled(account)
            || matches!(account.status, crate::models::AccountStatus::Forbidden)
        {
            return Some(AutoSwitchTriggerContext {
                rule: RULE_CURRENT_DISABLED,
                threshold,
                scope_mode: scope_mode.to_string(),
                hit_groups: Vec::new(),
            });
        }

        let summary = QuotaSummary::parse(account)?;
        if summary.is_forbidden {
            return Some(AutoSwitchTriggerContext {
                rule: RULE_CURRENT_QUOTA_FORBIDDEN,
                threshold,
                scope_mode: scope_mode.to_string(),
                hit_groups: Vec::new(),
            });
        }

        let mut hit_groups = Vec::new();
        for g in monitored_groups {
            if let Some(avg) = Self::account_group_average(account, g) {
                if avg <= threshold {
                    hit_groups.push(g.id.clone());
                }
            }
        }
        if hit_groups.is_empty() {
            return None;
        }
        Some(AutoSwitchTriggerContext {
            rule: RULE_GROUP_BELOW_THRESHOLD,
            threshold,
            scope_mode: scope_mode.to_string(),
            hit_groups,
        })
    }

    fn can_be_candidate(
        account: &IdeAccount,
        current_id: &str,
        threshold: i32,
        monitored_groups: &[AutoSwitchGroupDefinition],
    ) -> bool {
        if account.id == current_id {
            return false;
        }
        if matches!(account.status, crate::models::AccountStatus::Forbidden) {
            return false;
        }
        let Some(summary) = QuotaSummary::parse(account) else {
            return false;
        };
        if summary.is_forbidden || summary.models.is_empty() {
            return false;
        }
        for g in monitored_groups {
            match Self::account_group_average(account, g) {
                Some(avg) if avg < threshold => return false,
                None if !monitored_groups.is_empty() => {
                    // 候选必须能覆盖监控分组的指标
                    return false;
                }
                _ => {}
            }
        }
        true
    }

    fn resolve_groups(
        scope_mode: &str,
        selected_group_ids: &[String],
    ) -> Vec<AutoSwitchGroupDefinition> {
        let all = Self::default_groups();
        if scope_mode != AUTO_SWITCH_SCOPE_SELECTED_GROUPS {
            return all;
        }
        let selected: HashSet<&str> = selected_group_ids.iter().map(String::as_str).collect();
        if selected.is_empty() {
            return all;
        }
        let resolved: Vec<AutoSwitchGroupDefinition> = all
            .iter()
            .filter(|g| selected.contains(g.id.as_str()))
            .cloned()
            .collect();
        if resolved.is_empty() {
            Self::default_groups()
        } else {
            resolved
        }
    }

    fn resolve_account_pool(
        scope_mode: &str,
        selected_ids: &[String],
        accounts: &[IdeAccount],
    ) -> HashSet<String> {
        if scope_mode != AUTO_SWITCH_ACCOUNT_SCOPE_SELECTED {
            return accounts.iter().map(|a| a.id.clone()).collect();
        }
        let existing: HashSet<&str> = accounts.iter().map(|a| a.id.as_str()).collect();
        selected_ids
            .iter()
            .filter(|id| existing.contains(id.as_str()))
            .cloned()
            .collect()
    }

    pub async fn run_if_needed(
        db: &Database,
        app: Option<&AppHandle>,
    ) -> Result<AutoSwitchOutcome, String> {
        if AUTO_SWITCH_IN_PROGRESS.swap(true, Ordering::SeqCst) {
            tracing::info!("[AutoSwitch] 已在进行中，跳过本次");
            return Ok(AutoSwitchOutcome {
                triggered: false,
                from_account_id: None,
                to_account_id: None,
                rule: None,
                reason: Some("already_in_progress".to_string()),
            });
        }
        let result = Self::run_if_needed_inner(db, app).await;
        AUTO_SWITCH_IN_PROGRESS.store(false, Ordering::SeqCst);
        result
    }

    async fn run_if_needed_inner(
        db: &Database,
        app: Option<&AppHandle>,
    ) -> Result<AutoSwitchOutcome, String> {
        let settings = Self::load_settings(db);
        if !settings.enabled {
            return Ok(AutoSwitchOutcome {
                triggered: false,
                from_account_id: None,
                to_account_id: None,
                rule: None,
                reason: Some("disabled".to_string()),
            });
        }
        let threshold = Self::normalize_threshold(settings.threshold);
        let scope_mode = Self::normalize_scope_mode(&settings.scope_mode);
        let account_scope = Self::normalize_account_scope_mode(&settings.account_scope_mode);
        let monitored_groups = Self::resolve_groups(&scope_mode, &settings.selected_group_ids);

        // 取所有账号 + 解析当前账号（按平台逐个尝试）
        let accounts = db.get_all_ide_accounts().map_err(|e| e.to_string())?;
        let monitored_pool = Self::resolve_account_pool(
            &account_scope,
            &settings.selected_account_ids,
            &accounts,
        );

        let platforms = ["antigravity", "gemini", "codex"];
        for platform in platforms {
            let current_id = match ProviderCurrentService::get_current_account_id(db, platform) {
                Ok(Some(id)) => id,
                _ => continue,
            };
            if !monitored_pool.contains(&current_id) {
                continue;
            }
            let Some(current) = accounts.iter().find(|a| a.id == current_id) else {
                continue;
            };
            let Some(trigger) =
                Self::evaluate_trigger(current, threshold, &scope_mode, &monitored_groups)
            else {
                continue;
            };

            let mut candidates: Vec<IdeAccount> = accounts
                .iter()
                .filter(|a| {
                    a.origin_platform.eq_ignore_ascii_case(platform)
                        && monitored_pool.contains(&a.id)
                        && Self::can_be_candidate(a, &current_id, threshold, &monitored_groups)
                })
                .cloned()
                .collect();

            if candidates.is_empty() {
                tracing::warn!(
                    "[AutoSwitch] {} 触发 {} 但无可用候选",
                    platform,
                    trigger.rule
                );
                continue;
            }

            candidates.sort_by(|a, b| {
                let avg_a = QuotaSummary::parse(a)
                    .map(|s| s.average_percentage())
                    .unwrap_or(0.0);
                let avg_b = QuotaSummary::parse(b)
                    .map(|s| s.average_percentage())
                    .unwrap_or(0.0);
                avg_b
                    .partial_cmp(&avg_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.last_used.cmp(&b.last_used))
            });

            let target = candidates.into_iter().next().unwrap();
            tracing::info!(
                "[AutoSwitch] {} 触发切号 rule={} from={} to={}",
                platform,
                trigger.rule,
                current.email,
                target.email
            );

            let history = AccountSwitchHistoryItem {
                id: Uuid::new_v4().to_string(),
                ts: chrono::Utc::now().to_rfc3339(),
                trigger: "auto".to_string(),
                rule: Some(trigger.rule.to_string()),
                from_account_id: Some(current.id.clone()),
                from_email: Some(current.email.clone()),
                to_account_id: target.id.clone(),
                to_email: target.email.clone(),
                reason_json: Some(
                    serde_json::json!({
                        "threshold": trigger.threshold,
                        "scope_mode": trigger.scope_mode,
                        "hit_groups": trigger.hit_groups,
                    })
                    .to_string(),
                ),
            };
            let _ = db.append_account_switch_history(&history);

            // 执行切号
            let switch_result = if settings.hard_switch_enabled {
                crate::services::account_switch_executor::execute_hard_switch(
                    db, app, &target,
                )
                .await
            } else {
                crate::services::account_switch_executor::execute_soft_switch(
                    db, app, &target,
                )
                .await
            };

            match switch_result {
                Ok(()) => {
                    if let Some(app_handle) = app {
                        EventBus::emit_data_changed(
                            app_handle,
                            "ide_accounts",
                            "auto_switched",
                            "auto_switch.run",
                        );
                    }
                    return Ok(AutoSwitchOutcome {
                        triggered: true,
                        from_account_id: Some(current.id.clone()),
                        to_account_id: Some(target.id.clone()),
                        rule: Some(trigger.rule.to_string()),
                        reason: None,
                    });
                }
                Err(e) => {
                    tracing::warn!("[AutoSwitch] 切号执行失败: {}", e);
                    return Ok(AutoSwitchOutcome {
                        triggered: false,
                        from_account_id: Some(current.id.clone()),
                        to_account_id: Some(target.id.clone()),
                        rule: Some(trigger.rule.to_string()),
                        reason: Some(e),
                    });
                }
            }
        }

        Ok(AutoSwitchOutcome {
            triggered: false,
            from_account_id: None,
            to_account_id: None,
            rule: None,
            reason: Some("no_trigger".to_string()),
        })
    }
}
