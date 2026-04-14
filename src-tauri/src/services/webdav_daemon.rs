use crate::db::Database;
use crate::services::backup::BackupService;
use crate::services::webdav::{WebDavConfig, WebDavService};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};

pub struct WebDavDaemon {
    db: Arc<Database>,
    app_data_dir: PathBuf,
}

impl WebDavDaemon {
    pub fn new(db: Arc<Database>, app_data_dir: PathBuf) -> Self {
        Self { db, app_data_dir }
    }

    /// 在后台独立启动一个 Daemon，定期静默推送本地最新快照到 WebDAV
    pub fn start(self, interval_minutes: u64) {
        tauri::async_runtime::spawn(async move {
            info!(
                "守护进程已启动：WebDAV 配置状态漫游 (每 {} 分钟尝试推送备存)",
                interval_minutes
            );

            // 使用 interval 进行周期调度
            let mut interval = time::interval(Duration::from_secs(interval_minutes * 60));
            // 首次 Tick 会立刻执行，我们通常选择在服务刚启动时拉平一次
            interval.tick().await;

            loop {
                interval.tick().await;

                // 1. 读取配置文件 .webdav.json，如果存在且合法，则准备执行推送
                let config_path = self.app_data_dir.join(".webdav.json");
                if !config_path.exists() {
                    continue; // 尚未配置云端，跳过
                }

                let config_content = match std::fs::read_to_string(&config_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let config: WebDavConfig = match serde_json::from_str(&config_content) {
                    Ok(c) => c,
                    Err(_) => {
                        warn!("WebDAV 配置解析失败，请重新在设置中保存一次");
                        continue;
                    }
                };

                info!("WebDAV Daemon: 检测到配置，正在后台向云端同步...");

                // 2. 导出最新 DB 快照（这里不能阻塞 tokio executor 核心线程，使用 spawn_blocking）
                let db_clone = self.db.clone();
                let adf_clone = self.app_data_dir.clone();

                let json_data_res = tokio::task::spawn_blocking(
                    move || -> Result<String, crate::error::AppError> {
                        let backup_service = BackupService::new(&db_clone, adf_clone);
                        let backup_data = backup_service.export_config()?;
                        let str_data = serde_json::to_string_pretty(&backup_data).map_err(|e| {
                            crate::error::AppError::Other(anyhow::anyhow!("JSON error: {}", e))
                        })?;
                        Ok(str_data)
                    },
                )
                .await;

                let json_data = match json_data_res {
                    Ok(Ok(data)) => data,
                    _ => {
                        warn!("WebDAV Daemon: 导出配置快照失败");
                        continue;
                    }
                };

                // 3. 执行推送至 WebDAV
                let webdav_service = WebDavService::new();
                match webdav_service.push_backup(&config, &json_data).await {
                    Ok(_) => info!("WebDAV Daemon: 云端同步已成功"),
                    Err(e) => warn!("WebDAV Daemon: 云端同步失败 - {}", e),
                }
            }
        });
    }
}
