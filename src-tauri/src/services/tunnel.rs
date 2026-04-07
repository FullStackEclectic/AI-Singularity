use crate::error::AppError;
use regex::Regex;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};

// 全局隧道实例挂载点
lazy_static::lazy_static! {
    static ref CURRENT_TUNNEL: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    static ref TUNNEL_URL: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

/// 尝试一键暴起内网穿透隧道
pub async fn start_cloudflared_tunnel(app: AppHandle, port: u16) -> Result<(), AppError> {
    let mut child_guard = CURRENT_TUNNEL.lock().await;
    
    // 如果已有实例，先杀掉
    if let Some(mut existing) = child_guard.take() {
        let _ = existing.kill().await;
    }

    *TUNNEL_URL.lock().await = None;

    tracing::info!("🔗 正在启动边缘隧道 (cloudflared) 指向本地端口: {}", port);
    
    // 我们假设系统环境变量中存在 cloudflared，如无则交由用户安装或后续自动下载
    let mut cmd = Command::new("cloudflared");
    cmd.args(&["tunnel", "--url", &format!("http://127.0.0.1:{}", port)]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        AppError::Other(anyhow::anyhow!("启动 Cloudflared 失败 (请确认已安装并加入 PATH): {}", e))
    })?;

    let stderr = child.stderr.take().unwrap();
    let app_clone = app.clone();
    
    // 异步提取 stderr 中的 trycloudflare URL
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let re = Regex::new(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com").unwrap();
        
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::debug!("[Tunnel] {}", line);
            if let Some(caps) = re.captures(&line) {
                let url = caps.get(0).unwrap().as_str().to_string();
                tracing::info!("🔗 捕获到公网逃跃地址: {}", url);
                
                *TUNNEL_URL.lock().await = Some(url.clone());
                
                // 将信息推给前端雷达
                let _ = app_clone.emit("tunnel_url_ready", url);
            }
        }
    });

    *child_guard = Some(child);
    
    Ok(())
}

/// 关闭穿透隧道
pub async fn stop_cloudflared_tunnel() -> Result<(), AppError> {
    let mut child_guard = CURRENT_TUNNEL.lock().await;
    if let Some(mut existing) = child_guard.take() {
        let _ = existing.kill().await;
        *TUNNEL_URL.lock().await = None;
        tracing::info!("🔗 边缘隧道已掐断");
    }
    Ok(())
}

/// 获取当前连接状态
pub async fn get_tunnel_status() -> Result<Option<String>, AppError> {
    let url = TUNNEL_URL.lock().await.clone();
    Ok(url)
}

// ---------------- 暴露给前端的指令 ---------------- //

#[tauri::command]
pub async fn start_tunnel(app: AppHandle, port: u16) -> Result<(), AppError> {
    start_cloudflared_tunnel(app, port).await
}

#[tauri::command]
pub async fn stop_tunnel() -> Result<(), AppError> {
    stop_cloudflared_tunnel().await
}

#[tauri::command]
pub async fn filter_tunnel_status() -> Result<Option<String>, AppError> {
    get_tunnel_status().await
}
