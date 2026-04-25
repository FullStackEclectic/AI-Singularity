use crate::db::Database;
use crate::services::antigravity_ide::AntigravityIdeService;
use crate::services::codex_ide::CodexIdeService;
use crate::services::gemini_ide::GeminiIdeService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_CONCURRENT: usize = 5;
const REFRESH_RETRY_DELAY_SECS: u64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshTrigger {
    Auto,
    ManualBatch,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshStats {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub details: Vec<String>,
}

pub struct AccountRefreshOrchestrator;

impl AccountRefreshOrchestrator {
    pub async fn refresh_all(db: Arc<Database>, trigger: RefreshTrigger) -> RefreshStats {
        let trigger_label = match trigger {
            RefreshTrigger::Auto => "auto",
            RefreshTrigger::ManualBatch => "manual_batch",
        };
        let start = std::time::Instant::now();
        tracing::info!(
            "[AccountRefresh] 开始批量刷新所有 IDE 账号 (trigger={}, max_concurrent={})",
            trigger_label,
            MAX_CONCURRENT
        );

        let accounts = match db.get_all_ide_accounts() {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("[AccountRefresh] 加载账号失败: {}", e);
                return RefreshStats::default();
            }
        };

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
        let mut handles = Vec::new();

        for account in accounts {
            // Auto 模式跳过 forbidden / 禁用 / 代理禁用账号
            if matches!(trigger, RefreshTrigger::Auto) {
                if account.is_proxy_disabled
                    || matches!(account.status, crate::models::AccountStatus::Forbidden)
                {
                    continue;
                }
            }

            let platform = account.origin_platform.to_lowercase();
            if !matches!(platform.as_str(), "antigravity" | "gemini" | "codex") {
                continue;
            }

            let permit = semaphore.clone();
            let db_arc = db.clone();
            let account_id = account.id.clone();
            let email = account.email.clone();
            let platform_owned = platform;

            handles.push(tokio::spawn(async move {
                let _guard = permit.acquire().await.expect("semaphore closed");
                Self::refresh_one_with_retry(&db_arc, &platform_owned, &account_id, &email).await
            }));
        }

        let total = handles.len();
        let mut success = 0usize;
        let mut failed = 0usize;
        let mut details = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(Ok(())) => success += 1,
                Ok(Err(msg)) => {
                    failed += 1;
                    details.push(msg);
                }
                Err(join_err) => {
                    failed += 1;
                    details.push(format!("task panicked: {}", join_err));
                }
            }
        }

        tracing::info!(
            "[AccountRefresh] 批量刷新完成: total={}, success={}, failed={}, 耗时={}ms",
            total,
            success,
            failed,
            start.elapsed().as_millis()
        );

        RefreshStats {
            total,
            success,
            failed,
            details,
        }
    }

    async fn refresh_one_with_retry(
        db: &Database,
        platform: &str,
        account_id: &str,
        email: &str,
    ) -> Result<(), String> {
        match Self::refresh_one(db, platform, account_id).await {
            Ok(()) => Ok(()),
            Err(first_err) => {
                if crate::services::account_health::AccountHealthService::looks_like_invalid_grant(
                    &first_err,
                ) {
                    return Err(format!("{}({}): {}", email, account_id, first_err));
                }
                tracing::warn!(
                    "[AccountRefresh] 首次刷新失败，{}s 后重试: {} ({}) — {}",
                    REFRESH_RETRY_DELAY_SECS,
                    email,
                    account_id,
                    first_err
                );
                tokio::time::sleep(std::time::Duration::from_secs(REFRESH_RETRY_DELAY_SECS)).await;
                match Self::refresh_one(db, platform, account_id).await {
                    Ok(()) => Ok(()),
                    Err(second_err) => Err(format!("{}({}): {}", email, account_id, second_err)),
                }
            }
        }
    }

    pub(crate) async fn refresh_one(
        db: &Database,
        platform: &str,
        account_id: &str,
    ) -> Result<(), String> {
        match platform {
            "antigravity" => AntigravityIdeService::refresh_account(db, account_id)
                .await
                .map(|_| ()),
            "gemini" => GeminiIdeService::refresh_account(db, account_id)
                .await
                .map(|_| ()),
            "codex" => CodexIdeService::refresh_account(db, account_id)
                .await
                .map(|_| ()),
            _ => Err(format!("不支持的平台: {}", platform)),
        }
    }
}
