use crate::{db::Database, AppError};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_keys: i64,
    pub valid_keys: i64,
    pub invalid_keys: i64,
    pub unknown_keys: i64,
    pub total_platforms: i64,
    pub total_cost_usd: f64,
}

/// 获取总览统计数据
#[tauri::command]
pub async fn get_dashboard_stats(db: State<'_, Database>) -> Result<DashboardStats, AppError> {
    let total_keys: i64 = db.query_scalar("SELECT COUNT(*) FROM api_keys", &[])?;
    let valid_keys: i64 = db.query_scalar("SELECT COUNT(*) FROM api_keys WHERE status = 'valid'", &[])?;
    let invalid_keys: i64 = db.query_scalar(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('invalid','expired','banned')", &[])?;
    let unknown_keys: i64 = db.query_scalar(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('unknown','rate_limit')", &[])?;
    let total_platforms: i64 = db.query_scalar(
        "SELECT COUNT(DISTINCT platform) FROM api_keys", &[])?;
    let total_cost_usd: f64 = db.query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0.0) FROM usage_logs WHERE recorded_at >= date('now', 'start of month')",
        &[],
    ).unwrap_or(0.0);

    Ok(DashboardStats {
        total_keys, valid_keys, invalid_keys, unknown_keys, total_platforms, total_cost_usd,
    })
}
