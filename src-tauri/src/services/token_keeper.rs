use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use crate::services::account_health::AccountHealthService;
use crate::services::account_refresh_orchestrator::AccountRefreshOrchestrator;
use crate::services::event_bus::EventBus;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// 守护进程 tick 周期，远高于 cockpit 的 60s——但每次只做轻量扫描
const KEEPER_TICK_INTERVAL_SECS: u64 = 30;
/// Token 距离过期 < 此值时主动续期
const REFRESH_THRESHOLD_SECS: i64 = 300;
/// 同账号 60s 内不重复 refresh，避免节流耗尽
const PER_ACCOUNT_DEBOUNCE_SECS: u64 = 60;
/// 已启动标记（仅启动一次）
static STARTED: OnceLock<()> = OnceLock::new();
/// 节流表：account_id -> last_refresh Instant
static LAST_REFRESH: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
/// 最近一次 tick 元信息：用于看板
static LAST_TICK_META: OnceLock<Mutex<KeeperTickMeta>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct KeeperTickMeta {
    pub last_tick_at: Option<DateTime<Utc>>,
    pub last_rescues: usize,
}

pub struct TokenKeeper {
    db: Arc<Database>,
    app: AppHandle,
}

impl TokenKeeper {
    pub fn ensure_started(db: Arc<Database>, app: AppHandle) {
        if STARTED.set(()).is_err() {
            return;
        }
        let keeper = Self { db, app };
        tauri::async_runtime::spawn(async move {
            tracing::info!(
                "[TokenKeeper] 已启动：每 {}s 扫描一次，距离过期 < {}s 时主动续期",
                KEEPER_TICK_INTERVAL_SECS,
                REFRESH_THRESHOLD_SECS
            );
            let mut tick = tokio::time::interval(Duration::from_secs(KEEPER_TICK_INTERVAL_SECS));
            tick.tick().await;
            loop {
                tick.tick().await;
                keeper.run_tick().await;
            }
        });
    }

    async fn run_tick(&self) {
        let now = Utc::now();
        let accounts = match self.db.get_all_ide_accounts() {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("[TokenKeeper] 加载账号失败: {}", e);
                return;
            }
        };

        let mut rescued = 0usize;
        for account in accounts {
            if should_skip(&account) {
                continue;
            }
            let Some(remaining) = remaining_seconds(&account, now) else {
                continue;
            };
            if remaining > REFRESH_THRESHOLD_SECS {
                continue;
            }
            if !try_acquire_slot(&account.id) {
                tracing::debug!(
                    "[TokenKeeper] 节流跳过 id={} email={}",
                    account.id,
                    account.email
                );
                continue;
            }
            tracing::info!(
                "[TokenKeeper] 抢救 id={} email={} platform={} 剩余 {}s",
                account.id,
                account.email,
                account.origin_platform,
                remaining
            );
            self.refresh_account(&account).await;
            rescued += 1;
        }

        let meta = KeeperTickMeta {
            last_tick_at: Some(now),
            last_rescues: rescued,
        };
        if let Ok(mut guard) = last_tick_meta().lock() {
            *guard = meta;
        }
        if rescued > 0 {
            EventBus::emit_data_changed(
                &self.app,
                "ide_accounts",
                "token_keeper",
                "token_keeper.tick",
            );
        }
    }

    async fn refresh_account(&self, account: &IdeAccount) {
        let platform = account.origin_platform.to_lowercase();
        match AccountRefreshOrchestrator::refresh_one(&self.db, &platform, &account.id).await {
            Ok(()) => {
                AccountHealthService::try_clear_invalid_grant(&self.db, account);
            }
            Err(err) => {
                if AccountHealthService::looks_like_invalid_grant(&err) {
                    AccountHealthService::mark_invalid_grant(&self.db, &account.id, &err);
                } else {
                    tracing::warn!(
                        "[TokenKeeper] 抢救刷新失败 id={} err={}",
                        account.id,
                        err
                    );
                }
            }
        }
    }

    pub fn snapshot_overview(db: &Database) -> TokenHealthOverview {
        let now = Utc::now();
        let mut expiring_within_1h = 0usize;
        let mut already_expired = 0usize;
        if let Ok(accounts) = db.get_all_ide_accounts() {
            for account in accounts {
                if should_skip(&account) {
                    continue;
                }
                let Some(remaining) = remaining_seconds(&account, now) else {
                    continue;
                };
                if remaining <= 0 {
                    already_expired += 1;
                } else if remaining <= 3600 {
                    expiring_within_1h += 1;
                }
            }
        }

        let meta = last_tick_meta()
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        TokenHealthOverview {
            expiring_within_1h,
            already_expired,
            last_keeper_tick: meta.last_tick_at.map(|dt| dt.to_rfc3339()),
            last_keeper_rescues: meta.last_rescues,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenHealthOverview {
    pub expiring_within_1h: usize,
    pub already_expired: usize,
    pub last_keeper_tick: Option<String>,
    pub last_keeper_rescues: usize,
}

fn should_skip(account: &IdeAccount) -> bool {
    if account.is_proxy_disabled {
        return true;
    }
    if matches!(account.status, AccountStatus::Forbidden) {
        return true;
    }
    let platform = account.origin_platform.to_lowercase();
    !matches!(platform.as_str(), "antigravity" | "gemini" | "codex")
}

fn remaining_seconds(account: &IdeAccount, now: DateTime<Utc>) -> Option<i64> {
    if account.token.expires_in == 0 {
        return None;
    }
    let expires_at = account.token.updated_at
        + chrono::Duration::seconds(account.token.expires_in as i64);
    Some((expires_at - now).num_seconds())
}

fn last_tick_meta() -> &'static Mutex<KeeperTickMeta> {
    LAST_TICK_META.get_or_init(|| Mutex::new(KeeperTickMeta::default()))
}

fn last_refresh_map() -> &'static Mutex<HashMap<String, Instant>> {
    LAST_REFRESH.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 节流：同账号 PER_ACCOUNT_DEBOUNCE_SECS 内只接受一次 refresh
fn try_acquire_slot(account_id: &str) -> bool {
    let now = Instant::now();
    let mut guard = match last_refresh_map().lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if let Some(prev) = guard.get(account_id) {
        if now.duration_since(*prev) < Duration::from_secs(PER_ACCOUNT_DEBOUNCE_SECS) {
            return false;
        }
    }
    guard.insert(account_id.to_string(), now);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OAuthToken;

    fn sample_account(remaining_secs: i64) -> IdeAccount {
        let now = Utc::now();
        let token_updated_at = now - chrono::Duration::seconds(3600 - remaining_secs);
        IdeAccount {
            id: "acct-1".to_string(),
            email: "test@example.com".to_string(),
            origin_platform: "codex".to_string(),
            token: OAuthToken {
                access_token: "tok".to_string(),
                refresh_token: "ref".to_string(),
                expires_in: 3600,
                token_type: "Bearer".to_string(),
                updated_at: token_updated_at,
            },
            status: AccountStatus::Active,
            disabled_reason: None,
            is_proxy_disabled: false,
            created_at: now,
            updated_at: now,
            last_used: now,
            device_profile: None,
            quota_json: None,
            project_id: None,
            meta_json: None,
            label: None,
            tags: Vec::new(),
            disabled_at: None,
            fingerprint_id: None,
            quota_error_json: None,
        }
    }

    #[test]
    fn remaining_seconds_correctly_reflects_token_expiry() {
        let account = sample_account(120);
        let now = Utc::now();
        let remaining = remaining_seconds(&account, now).expect("remaining");
        assert!(remaining <= 121 && remaining >= 119, "got {}", remaining);
    }

    #[test]
    fn skip_forbidden_accounts() {
        let mut account = sample_account(60);
        account.status = AccountStatus::Forbidden;
        assert!(should_skip(&account));
    }

    #[test]
    fn skip_unsupported_platform() {
        let mut account = sample_account(60);
        account.origin_platform = "unknown".to_string();
        assert!(should_skip(&account));
    }

    #[test]
    fn debounce_blocks_repeat_refresh() {
        let id = "debounce-acct";
        // Drain any earlier registrations first.
        let _ = last_refresh_map()
            .lock()
            .ok()
            .map(|mut g| g.remove(id));
        assert!(try_acquire_slot(id), "first call should succeed");
        assert!(!try_acquire_slot(id), "second call within window should be blocked");
    }
}
