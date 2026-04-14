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
    let valid_keys: i64 =
        db.query_scalar("SELECT COUNT(*) FROM api_keys WHERE status = 'valid'", &[])?;
    let invalid_keys: i64 = db.query_scalar(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('invalid','expired','banned')",
        &[],
    )?;
    let unknown_keys: i64 = db.query_scalar(
        "SELECT COUNT(*) FROM api_keys WHERE status IN ('unknown','rate_limit')",
        &[],
    )?;
    let total_platforms: i64 =
        db.query_scalar("SELECT COUNT(DISTINCT platform) FROM api_keys", &[])?;
    let total_cost_usd: f64 = db.query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0.0) FROM usage_logs WHERE recorded_at >= date('now', 'start of month')",
        &[],
    ).unwrap_or(0.0);

    Ok(DashboardStats {
        total_keys,
        valid_keys,
        invalid_keys,
        unknown_keys,
        total_platforms,
        total_cost_usd,
    })
}

/// 前端图表所需的 Token 用量聚合行（通用格式）
#[derive(Debug, Serialize)]
pub struct TokenStatRow {
    /// 分组名（client_app / model_name / platform 等）
    pub name: String,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

/// 全量 Token 用量聚合数据（支持多维度下钻）
#[derive(Debug, Serialize)]
pub struct TokenUsageStats {
    /// 按工具/客户端分组（对应前端 by_app）
    pub by_app: Vec<TokenStatRow>,
    /// 按模型分组
    pub by_model: Vec<TokenStatRow>,
    /// 按平台分组
    pub by_platform: Vec<TokenStatRow>,
}

/// 获取 Token 消耗审计聚类排行（多维度）
#[tauri::command]
pub async fn get_token_usage_stats(db: State<'_, Database>) -> Result<TokenUsageStats, AppError> {
    let by_app: Vec<TokenStatRow> = db
        .query_rows(
            "SELECT client_app,
                COALESCE(SUM(total_tokens), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0)
         FROM token_usage_records
         GROUP BY client_app
         ORDER BY SUM(total_tokens) DESC",
            &[],
            |row| {
                Ok(TokenStatRow {
                    name: row.get(0)?,
                    total_tokens: row.get(1)?,
                    prompt_tokens: row.get(2)?,
                    completion_tokens: row.get(3)?,
                })
            },
        )
        .unwrap_or_default();

    let by_model: Vec<TokenStatRow> = db
        .query_rows(
            "SELECT model_name,
                COALESCE(SUM(total_tokens), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0)
         FROM token_usage_records
         GROUP BY model_name
         ORDER BY SUM(total_tokens) DESC",
            &[],
            |row| {
                Ok(TokenStatRow {
                    name: row.get(0)?,
                    total_tokens: row.get(1)?,
                    prompt_tokens: row.get(2)?,
                    completion_tokens: row.get(3)?,
                })
            },
        )
        .unwrap_or_default();

    let by_platform: Vec<TokenStatRow> = db
        .query_rows(
            "SELECT platform,
                COALESCE(SUM(total_tokens), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0)
         FROM token_usage_records
         GROUP BY platform
         ORDER BY SUM(total_tokens) DESC",
            &[],
            |row| {
                Ok(TokenStatRow {
                    name: row.get(0)?,
                    total_tokens: row.get(1)?,
                    prompt_tokens: row.get(2)?,
                    completion_tokens: row.get(3)?,
                })
            },
        )
        .unwrap_or_default();

    Ok(TokenUsageStats {
        by_app,
        by_model,
        by_platform,
    })
}
