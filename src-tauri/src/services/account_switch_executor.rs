use crate::db::Database;
use crate::models::IdeAccount;
use crate::services::antigravity_ide::AntigravityIdeService;
use crate::services::codex_ide::CodexIdeService;
use crate::services::event_bus::EventBus;
use crate::services::gemini_ide::GeminiIdeService;
use crate::services::ide_injector::IdeInjector;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
const ANTIGRAVITY_PROCESS_NAMES: &[&str] = &["Antigravity.exe"];
#[cfg(target_os = "macos")]
const ANTIGRAVITY_PROCESS_NAMES: &[&str] = &["Antigravity"];
#[cfg(target_os = "linux")]
const ANTIGRAVITY_PROCESS_NAMES: &[&str] = &["antigravity"];

pub async fn execute_soft_switch(
    db: &Database,
    app: Option<&AppHandle>,
    target: &IdeAccount,
) -> Result<(), String> {
    refresh_target_token(db, target).await?;
    IdeInjector::execute_injection(target).map_err(|e| e.to_string())?;
    if let Some(app_handle) = app {
        EventBus::emit_data_changed(
            app_handle,
            "ide_accounts",
            "soft_switch",
            "switch_executor.soft",
        );
    }
    tracing::info!(
        "[SwitchExecutor] 软切号完成: target={} ({})",
        target.email,
        target.origin_platform
    );
    Ok(())
}

pub async fn execute_hard_switch(
    db: &Database,
    app: Option<&AppHandle>,
    target: &IdeAccount,
) -> Result<(), String> {
    let platform = target.origin_platform.to_lowercase();

    refresh_target_token(db, target).await?;

    if platform == "antigravity" {
        if let Err(e) = close_processes(ANTIGRAVITY_PROCESS_NAMES) {
            tracing::warn!("[SwitchExecutor] 关闭 Antigravity 进程失败（忽略继续）: {}", e);
        }
    }

    apply_fingerprint_if_bound(db, target);

    IdeInjector::execute_injection(target).map_err(|e| e.to_string())?;

    if platform == "antigravity" {
        if let Err(e) = launch_antigravity() {
            tracing::warn!("[SwitchExecutor] 重启 Antigravity 失败（忽略继续）: {}", e);
        }
    }

    if let Some(app_handle) = app {
        EventBus::emit_data_changed(
            app_handle,
            "ide_accounts",
            "hard_switch",
            "switch_executor.hard",
        );
    }
    tracing::info!(
        "[SwitchExecutor] 硬切号完成: target={} ({})",
        target.email,
        target.origin_platform
    );
    Ok(())
}

async fn refresh_target_token(db: &Database, target: &IdeAccount) -> Result<(), String> {
    let platform = target.origin_platform.to_lowercase();
    match platform.as_str() {
        "antigravity" => AntigravityIdeService::refresh_account(db, &target.id)
            .await
            .map(|_| ()),
        "gemini" => GeminiIdeService::refresh_account(db, &target.id)
            .await
            .map(|_| ()),
        "codex" => CodexIdeService::refresh_account(db, &target.id)
            .await
            .map(|_| ()),
        _ => Ok(()),
    }
}

fn apply_fingerprint_if_bound(db: &Database, target: &IdeAccount) {
    let Some(fp_id) = target.fingerprint_id.as_deref() else {
        return;
    };
    if fp_id.is_empty() || fp_id == "original" {
        return;
    }
    let Ok(Some(fp)) = db.get_device_fingerprint(fp_id) else {
        tracing::warn!(
            "[SwitchExecutor] 账号 {} 绑定指纹 {} 未找到，跳过指纹注入",
            target.email,
            fp_id
        );
        return;
    };
    if let Err(e) = crate::services::device_fingerprint::DeviceFingerprintService::write_to_storage(
        &target.origin_platform,
        &fp,
    ) {
        tracing::warn!(
            "[SwitchExecutor] 写入设备指纹失败: target={} fp={} err={}",
            target.email,
            fp_id,
            e
        );
    } else {
        tracing::info!(
            "[SwitchExecutor] 已为账号 {} 写入设备指纹 {}",
            target.email,
            fp.name
        );
    }
}

#[cfg(target_os = "windows")]
fn close_processes(names: &[&str]) -> Result<(), String> {
    use std::process::Command;
    for name in names {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", name])
            .output()
            .map_err(|e| format!("taskkill {} 失败: {}", name, e))?;
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn close_processes(names: &[&str]) -> Result<(), String> {
    use std::process::Command;
    for name in names {
        let _ = Command::new("pkill")
            .args(["-x", name])
            .output()
            .map_err(|e| format!("pkill {} 失败: {}", name, e))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_antigravity() -> Result<(), String> {
    use std::process::Command;
    let appdata = std::env::var("LOCALAPPDATA")
        .map_err(|_| "缺少 LOCALAPPDATA 环境变量".to_string())?;
    let exe = std::path::PathBuf::from(appdata)
        .join("Programs")
        .join("Antigravity")
        .join("Antigravity.exe");
    if !exe.exists() {
        return Err(format!("未找到 Antigravity 可执行文件: {}", exe.display()));
    }
    Command::new(&exe)
        .spawn()
        .map_err(|e| format!("启动 Antigravity 失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_antigravity() -> Result<(), String> {
    use std::process::Command;
    Command::new("open")
        .args(["-a", "Antigravity"])
        .spawn()
        .map_err(|e| format!("启动 Antigravity 失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_antigravity() -> Result<(), String> {
    use std::process::Command;
    Command::new("antigravity")
        .spawn()
        .map_err(|e| format!("启动 Antigravity 失败: {}", e))?;
    Ok(())
}
