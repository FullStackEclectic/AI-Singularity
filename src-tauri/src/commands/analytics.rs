use crate::db::Database;
use crate::error::AppResult;
use crate::services::analytics::{AnalyticsService, DashboardMetrics};
use tauri::State;

#[tauri::command]
pub async fn get_dashboard_metrics(
    days: u32,
    db: State<'_, Database>,
) -> AppResult<DashboardMetrics> {
    AnalyticsService::get_metrics(&db, days)
}
