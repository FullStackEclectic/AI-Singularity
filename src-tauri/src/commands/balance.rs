use crate::db::Database;
use crate::{models::Balance, AppError};
use tauri::State;

/// 获取所有账号余额（从缓存）
#[tauri::command]
pub async fn get_all_balances(db: State<'_, Database>) -> Result<Vec<Balance>, AppError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT k.id, k.platform, b.balance_usd, b.balance_cny, b.total_usage_usd,
                b.quota_remaining, b.quota_reset_at, b.synced_at
         FROM api_keys k
         LEFT JOIN balances b ON k.id = b.key_id
         ORDER BY k.created_at DESC",
    )?;

    let balances = stmt
        .query_map([], |row| {
            use crate::models::Platform;
            let platform =
                serde_json::from_str::<Platform>(&format!("\"{}\"", row.get::<_, String>(1)?))
                    .unwrap_or(Platform::Custom);
            Ok(Balance {
                key_id: row.get(0)?,
                platform,
                balance_usd: row.get(2)?,
                balance_cny: row.get(3)?,
                total_usage_usd: row.get(4)?,
                quota_remaining: row.get(5)?,
                quota_reset_at: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| s.parse().ok()),
                synced_at: row
                    .get::<_, Option<String>>(7)?
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(chrono::Utc::now()),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(balances)
}

/// 刷新指定平台的余额
#[tauri::command]
pub async fn get_platform_balance(
    db: State<'_, Database>,
    key_id: String,
) -> Result<Balance, AppError> {
    // TODO: Phase 1 - 实现各平台余额查询 API 调用
    Err(AppError::Other(anyhow::anyhow!("余额查询功能开发中")))
}
