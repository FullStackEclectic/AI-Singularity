use crate::db::Database;
use crate::error::AppError;
use crate::models::{BalanceSnapshot, BalanceSummary};
use crate::services::balance_tracker::BalanceTracker;
use tauri::State;

/// 获取所有 Provider 最新余额汇总（Dashboard 展示）
#[tauri::command]
pub async fn get_balance_summaries(
    db: State<'_, Database>,
) -> Result<Vec<BalanceSummary>, AppError> {
    BalanceTracker::new(&*db).get_summaries()
}

/// 刷新所有 Provider 余额（手动触发）
#[tauri::command]
pub async fn refresh_provider_balances(
    db: State<'_, Database>,
) -> Result<Vec<BalanceSnapshot>, AppError> {
    BalanceTracker::new(&*db).refresh_all().await
}

/// 刷新单个 Provider 余额
#[tauri::command]
pub async fn refresh_provider_balance(
    provider_id: String,
    db: State<'_, Database>,
) -> Result<BalanceSnapshot, AppError> {
    BalanceTracker::new(&*db).refresh_one(&provider_id).await
}

/// 获取某 Provider 的历史余额趋势
#[tauri::command]
pub async fn get_balance_history(
    provider_id: String,
    limit: Option<u32>,
    db: State<'_, Database>,
) -> Result<Vec<BalanceSnapshot>, AppError> {
    BalanceTracker::new(&*db).get_history(&provider_id, limit.unwrap_or(30))
}
