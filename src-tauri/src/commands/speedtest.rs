use crate::error::AppResult;
use crate::models::SpeedTestResult;
use crate::services::speedtest::SpeedTestService;

#[tauri::command]
pub async fn run_speedtest() -> AppResult<Vec<SpeedTestResult>> {
    let results = SpeedTestService::test_all().await;
    Ok(results)
}
