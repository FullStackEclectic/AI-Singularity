use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};

const INVALID_GRANT_REASON_PREFIX: &str = "invalid_grant";

pub struct AccountHealthService;

impl AccountHealthService {
    pub fn is_invalid_grant_disabled(account: &IdeAccount) -> bool {
        matches!(account.status, AccountStatus::Forbidden)
            && account
                .disabled_reason
                .as_deref()
                .is_some_and(|r| r.starts_with(INVALID_GRANT_REASON_PREFIX))
    }

    pub fn looks_like_invalid_grant(error_msg: &str) -> bool {
        let lower = error_msg.to_ascii_lowercase();
        lower.contains("invalid_grant")
            || lower.contains("invalid grant")
            || lower.contains("refresh token expired")
            || lower.contains("token has been expired")
    }

    pub fn mark_invalid_grant(db: &Database, account_id: &str, error_msg: &str) {
        let reason = format!("{}: {}", INVALID_GRANT_REASON_PREFIX, error_msg);
        if let Err(e) = db.mark_ide_account_disabled(account_id, &reason) {
            tracing::warn!(
                "[AccountHealth] mark_invalid_grant 写入失败 id={} err={}",
                account_id,
                e
            );
        } else {
            tracing::warn!(
                "[AccountHealth] 账号 {} 触发 invalid_grant，已标记禁用: {}",
                account_id,
                error_msg
            );
        }
    }

    pub fn try_clear_invalid_grant(db: &Database, account: &IdeAccount) {
        if !Self::is_invalid_grant_disabled(account) {
            return;
        }
        match db.clear_ide_account_disabled(&account.id) {
            Ok(rows) if rows > 0 => {
                tracing::info!(
                    "[AccountHealth] 账号 {} 配额刷新成功，自动解除 invalid_grant 禁用",
                    account.email
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    "[AccountHealth] try_clear_invalid_grant 失败 id={} err={}",
                    account.id,
                    e
                );
            }
        }
    }

    pub fn record_quota_error(
        db: &Database,
        account_id: &str,
        code: Option<u16>,
        message: &str,
    ) {
        let payload = serde_json::json!({
            "code": code,
            "message": message,
            "timestamp": chrono::Utc::now().timestamp(),
        });
        if let Err(e) = db.update_ide_account_quota_error(account_id, Some(&payload.to_string())) {
            tracing::warn!(
                "[AccountHealth] record_quota_error 写入失败 id={} err={}",
                account_id,
                e
            );
        }
    }

    pub fn clear_quota_error(db: &Database, account_id: &str) {
        if let Err(e) = db.update_ide_account_quota_error(account_id, None) {
            tracing::warn!(
                "[AccountHealth] clear_quota_error 写入失败 id={} err={}",
                account_id,
                e
            );
        }
    }

    pub fn list_disabled(db: &Database) -> Vec<IdeAccount> {
        db.get_all_ide_accounts()
            .unwrap_or_default()
            .into_iter()
            .filter(|acc| matches!(acc.status, AccountStatus::Forbidden))
            .collect()
    }
}
