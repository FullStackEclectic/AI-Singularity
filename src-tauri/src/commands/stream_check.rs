use crate::error::AppResult;
use crate::models::StreamCheckResult;
use crate::services::stream_check::StreamCheckService;
use crate::db::Database;
use tauri::State;

#[tauri::command]
pub async fn stream_check_provider(
    provider_id: String,
    db: State<'_, Database>,
) -> AppResult<StreamCheckResult> {
    StreamCheckService::run_check(&*db, &provider_id).await
}
