use crate::db::Database;
use crate::error::AppResult;
use crate::services::backup::BackupService;
use crate::services::webdav::{WebDavConfig, WebDavService};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn webdav_test_connection(config: WebDavConfig) -> AppResult<()> {
    let service = WebDavService::new();
    service.test_connection(&config).await
}

#[tauri::command]
pub async fn webdav_save_config(config: WebDavConfig, app: AppHandle) -> AppResult<()> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    let config_path = app_data_dir.join(".webdav.json");
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)?;
    }
    let json_str = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, json_str)?;
    // Reload daemon inside later if needed
    Ok(())
}

#[tauri::command]
pub async fn webdav_push(
    config: WebDavConfig,
    app: AppHandle,
    db: State<'_, Database>,
) -> AppResult<()> {
    // 1. 获取当前最新设置
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();

    // 我们必须在阻塞线程或提前抽取这部分数据以避免在异步块中产生冲突，但因为它是同步的方法，使用 spawn_blocking 较好
    let state_db = db.inner().clone();

    let json_data = tokio::task::spawn_blocking(move || {
        let backup_service = BackupService::new(&state_db, app_data_dir);
        let backup_data = backup_service.export_config()?;
        serde_json::to_string_pretty(&backup_data)
            .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e.to_string())))
    })
    .await
    .map_err(|_| crate::error::AppError::Other(anyhow::anyhow!("Task panic".to_string())))??;

    // 2. 上传
    let service = WebDavService::new();
    service.push_backup(&config, &json_data).await?;

    Ok(())
}

#[tauri::command]
pub async fn webdav_pull(
    config: WebDavConfig,
    app: AppHandle,
    db: State<'_, Database>,
) -> AppResult<()> {
    // 1. 下载
    let service = WebDavService::new();
    let json_data = service.pull_backup(&config).await?;

    // 2. 导入
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    let state_db = db.inner().clone();

    tokio::task::spawn_blocking(move || {
        let backup_service = BackupService::new(&state_db, app_data_dir);
        backup_service.import_config(&json_data)
    })
    .await
    .map_err(|_| crate::error::AppError::Other(anyhow::anyhow!("Task panic".to_string())))??;

    // 3. 刷新托盘等全局状态
    crate::tray::update_tray_menu(&app);

    Ok(())
}
