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
    pub total_cost_usd: f64, // 本月预估消耗
}

/// 获取总览统计数据
#[tauri::command]
pub async fn get_dashboard_stats(db: State<'_, Database>) -> Result<DashboardStats, AppError> {
    let conn = db.conn();

    let total_keys: i64 =
        conn.query_row("SELECT COUNT(*) FROM api_keys", [], |r| r.get(0))?;

    let valid_keys: i64 = conn.query_row(
        "SELECT COUNT(*) FROM api_keys WHERE status = 'valid'",
        [],
        |r| r.get(0),
    )?;

    let invalid_keys: i64 = conn.query_row(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('invalid','expired','banned')",
        [],
        |r| r.get(0),
    )?;

    let unknown_keys: i64 = conn.query_row(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('unknown','rate_limit')",
        [],
        |r| r.get(0),
    )?;

    let total_platforms: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT platform) FROM api_keys",
        [],
        |r| r.get(0),
    )?;

    // 本月消耗（从用量日志累计）
    let total_cost_usd: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM usage_logs
             WHERE recorded_at >= date('now', 'start of month')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    Ok(DashboardStats {
        total_keys,
        valid_keys,
        invalid_keys,
        unknown_keys,
        total_platforms,
        total_cost_usd,
    })
}
