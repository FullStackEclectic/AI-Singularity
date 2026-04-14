use crate::models::EngineConfig;
use crate::{db::Database, AppError};
use lazy_static::lazy_static;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use tauri::{AppHandle, Manager, State};

lazy_static! {
    pub static ref ENGINE_CONFIG: RwLock<EngineConfig> = RwLock::new(EngineConfig::default());
}

static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);
static PROXY_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(8765);

#[derive(Debug, Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub endpoint: String,
}

/// 启动本地代理
#[tauri::command]
pub async fn start_proxy(
    app: AppHandle,
    _db: State<'_, Database>,
    port: Option<u16>,
) -> Result<ProxyStatus, AppError> {
    let port = port.unwrap_or(8765);

    if PROXY_RUNNING.load(Ordering::SeqCst) {
        return Ok(ProxyStatus {
            running: true,
            port,
            endpoint: format!("http://127.0.0.1:{}/v1", port),
        });
    }

    // 通过 AppHandle 获取数据库路径
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(anyhow::anyhow!("无法获取数据目录: {}", e)))?
        .join("data.db");

    let db_arc = Arc::new(Database::new(&db_path)?);

    PROXY_RUNNING.store(true, Ordering::SeqCst);
    PROXY_PORT.store(port, Ordering::SeqCst);

    let server = crate::proxy::server::ProxyServer::new(db_arc, port);

    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("代理服务器错误: {}", e);
        }
        PROXY_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(ProxyStatus {
        running: true,
        port,
        endpoint: format!("http://127.0.0.1:{}/v1", port),
    })
}

/// 获取代理状态
#[tauri::command]
pub async fn get_proxy_status(port: Option<u16>) -> Result<ProxyStatus, AppError> {
    let port = port.unwrap_or_else(|| PROXY_PORT.load(Ordering::SeqCst));
    Ok(ProxyStatus {
        running: PROXY_RUNNING.load(Ordering::SeqCst),
        port,
        endpoint: format!("http://127.0.0.1:{}/v1", port),
    })
}

/// 同步前端 Proxy Engine 配置到后端运行时
#[tauri::command]
pub async fn sync_proxy_engine_config(config: EngineConfig) -> Result<(), AppError> {
    if let Ok(mut lock) = ENGINE_CONFIG.write() {
        tracing::info!("📡 收到前端高级引擎配置同步: {:?}", config);
        *lock = config;
    }
    Ok(())
}
