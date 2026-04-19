use super::WakeupTask;
use crate::db::Database;
use crate::services::ide_injector::IdeInjector;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub(super) struct WakeupAttemptOutcome {
    pub(super) execution: WakeupExecutionResult,
    pub(super) attempts: usize,
    pub(super) category: String,
}

#[derive(Debug, Clone)]
struct ResolvedWakeupClientProfile {
    requested_mode: String,
    fallback_mode: String,
    effective_mode: String,
    runtime_args: String,
    gateway_mode: String,
    gateway_transport: String,
    gateway_routing: String,
    gateway_version_hint: String,
    fallback_reason: Option<String>,
}

pub(super) struct WakeupExecutionResult {
    pub(super) success: bool,
    pub(super) message: String,
}

pub(super) fn normalize_client_version_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "official_stable" | "stable" => "official_stable".to_string(),
        "official_preview" | "preview" | "beta" => "official_preview".to_string(),
        "official_legacy" | "legacy" | "v1_legacy" => "official_legacy".to_string(),
        _ => "auto".to_string(),
    }
}

pub(super) fn execute_wakeup_task_with_retry(
    db: &Database,
    task: &WakeupTask,
    retry_failed_times: usize,
    cancel_flag: Option<&AtomicBool>,
) -> WakeupAttemptOutcome {
    let mut execution = execute_wakeup_task(db, task, cancel_flag);
    let mut attempts = 1usize;
    while !execution.success && attempts <= retry_failed_times.min(5) {
        if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            break;
        }
        attempts += 1;
        execution = execute_wakeup_task(db, task, cancel_flag);
    }
    let category = classify_execution_category(execution.success, &execution.message);
    WakeupAttemptOutcome {
        execution,
        attempts,
        category,
    }
}

pub(super) fn apply_attempt_outcome_to_task(
    task: &mut WakeupTask,
    outcome: &WakeupAttemptOutcome,
    run_at: &str,
) -> bool {
    task.last_run_at = Some(run_at.to_string());
    task.updated_at = run_at.to_string();
    task.last_status = Some(if outcome.execution.success {
        "success".to_string()
    } else {
        "error".to_string()
    });
    task.last_category = Some(outcome.category.clone());

    if outcome.execution.success {
        task.consecutive_failures = 0;
        task.last_message = Some(if outcome.attempts > 1 {
            format!(
                "执行成功（共尝试 {} 次）。{}",
                outcome.attempts, outcome.execution.message
            )
        } else {
            outcome.execution.message.clone()
        });
        return false;
    }

    task.consecutive_failures = task.consecutive_failures.saturating_add(1);
    let mut message = if outcome.attempts > 1 {
        format!(
            "执行失败（共尝试 {} 次）。{}",
            outcome.attempts, outcome.execution.message
        )
    } else {
        outcome.execution.message.clone()
    };
    let should_pause =
        task.pause_after_failures > 0 && task.consecutive_failures >= task.pause_after_failures;
    if should_pause {
        task.enabled = false;
        message.push_str(&format!(
            " 已连续失败 {} 次，任务已自动暂停。",
            task.consecutive_failures
        ));
    }
    task.last_message = Some(message);
    should_pause
}

pub(super) fn notify_task_auto_paused(app: &AppHandle, task: &WakeupTask) {
    let title = format!(
        "Wakeup 任务已自动暂停: {}",
        if task.name.trim().is_empty() {
            "未命名任务"
        } else {
            task.name.as_str()
        }
    );
    let body = task
        .last_message
        .clone()
        .unwrap_or_else(|| "任务连续失败，已自动暂停。".to_string());
    let _ = app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show();
}

fn platform_client_family(origin_platform: &str) -> &'static str {
    let platform = origin_platform.trim().to_ascii_lowercase();
    if platform.contains("gemini") {
        "gemini"
    } else if platform.contains("codex") {
        "codex"
    } else {
        "generic"
    }
}

fn is_mode_supported_for_family(family: &str, mode: &str) -> bool {
    if mode == "auto" {
        return true;
    }
    match family {
        "gemini" | "codex" => matches!(
            mode,
            "official_stable" | "official_preview" | "official_legacy"
        ),
        _ => mode == "official_legacy",
    }
}

fn profile_fields_for_mode(
    family: &str,
    mode: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match (family, mode) {
        ("gemini", "official_stable") => (
            "--client-channel stable",
            "strict",
            "oauth_refresh",
            "gemini_official",
            "Gemini 官方稳定通道",
        ),
        ("gemini", "official_preview") => (
            "--client-channel preview --enable-preview",
            "compat_preview",
            "oauth_refresh",
            "gemini_preview",
            "Gemini 官方预览通道",
        ),
        ("gemini", "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "gemini_legacy",
            "Gemini 旧版兼容链路",
        ),
        ("codex", "official_stable") => (
            "--channel stable",
            "strict",
            "oauth_token",
            "codex_official",
            "Codex 官方稳定通道",
        ),
        ("codex", "official_preview") => (
            "--channel preview --enable-beta",
            "compat_preview",
            "oauth_token",
            "codex_preview",
            "Codex 官方预览通道",
        ),
        ("codex", "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "codex_legacy",
            "Codex 旧版兼容链路",
        ),
        (_, "official_legacy") => (
            "--legacy-auth-flow",
            "legacy_compat",
            "oauth_legacy",
            "generic_legacy",
            "通用旧版兼容链路",
        ),
        _ => ("", "auto", "auto", "auto", "自动跟随当前官方客户端"),
    }
}

fn resolve_task_client_profile(
    task: &WakeupTask,
    origin_platform: &str,
) -> ResolvedWakeupClientProfile {
    let requested_mode = normalize_client_version_mode(&task.client_version_mode);
    let fallback_mode = normalize_client_version_mode(&task.client_version_fallback_mode);
    let family = platform_client_family(origin_platform);

    let (effective_mode, fallback_reason) = if is_mode_supported_for_family(family, &requested_mode)
    {
        (requested_mode.clone(), None)
    } else if is_mode_supported_for_family(family, &fallback_mode) {
        (
            fallback_mode.clone(),
            Some(format!(
                "平台 {} 不支持 {}，已回退到 {}",
                origin_platform, requested_mode, fallback_mode
            )),
        )
    } else {
        (
            "auto".to_string(),
            Some(format!(
                "平台 {} 不支持 {} / {}，已强制回退到 auto",
                origin_platform, requested_mode, fallback_mode
            )),
        )
    };

    let (runtime_args, gateway_mode, gateway_transport, gateway_routing, gateway_version_hint) =
        profile_fields_for_mode(family, &effective_mode);

    ResolvedWakeupClientProfile {
        requested_mode,
        fallback_mode,
        effective_mode,
        runtime_args: runtime_args.to_string(),
        gateway_mode: gateway_mode.to_string(),
        gateway_transport: gateway_transport.to_string(),
        gateway_routing: gateway_routing.to_string(),
        gateway_version_hint: gateway_version_hint.to_string(),
        fallback_reason,
    }
}

fn execute_wakeup_task(
    db: &Database,
    task: &WakeupTask,
    cancel_flag: Option<&AtomicBool>,
) -> WakeupExecutionResult {
    let account = match db
        .get_all_ide_accounts()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.id == task.account_id))
    {
        Some(account) => account,
        None => {
            return WakeupExecutionResult {
                success: false,
                message: "未找到绑定的 IDE 账号。".to_string(),
            }
        }
    };

    if let Err(err) = IdeInjector::execute_injection(&account) {
        return WakeupExecutionResult {
            success: false,
            message: format!("账号注入失败: {}", err),
        };
    }

    let profile = resolve_task_client_profile(task, &account.origin_platform);
    let command = render_command_template(task, &account.email, &profile);
    let profile_desc = if let Some(reason) = &profile.fallback_reason {
        format!(
            "；客户端模式 {}（请求 {}，{}）",
            profile.effective_mode, profile.requested_mode, reason
        )
    } else {
        format!("；客户端模式 {}", profile.effective_mode)
    };
    match run_shell_command(&command, task.timeout_seconds, cancel_flag) {
        Ok(output) => WakeupExecutionResult {
            success: true,
            message: format!(
                "已执行命令：{}{}{}",
                command,
                output
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("；输出：{}", value))
                    .unwrap_or_default(),
                profile_desc,
            ),
        },
        Err(err) => WakeupExecutionResult {
            success: false,
            message: format!("执行命令失败：{}；命令：{}{}", err, command, profile_desc),
        },
    }
}

fn render_command_template(
    task: &WakeupTask,
    email: &str,
    profile: &ResolvedWakeupClientProfile,
) -> String {
    let had_runtime_placeholder = task.command_template.contains("{client_runtime_args}");
    let rendered = task
        .command_template
        .replace("{model}", &task.model)
        .replace("{prompt}", &task.prompt)
        .replace("{account_id}", &task.account_id)
        .replace("{email}", email)
        .replace("{client_version_mode}", &profile.effective_mode)
        .replace(
            "{client_version_mode_requested}",
            &profile.requested_mode,
        )
        .replace(
            "{client_version_fallback_mode}",
            &profile.fallback_mode,
        )
        .replace("{client_runtime_args}", &profile.runtime_args)
        .replace("{gateway_mode}", &profile.gateway_mode)
        .replace("{gateway_transport}", &profile.gateway_transport)
        .replace("{gateway_routing}", &profile.gateway_routing)
        .replace("{gateway_version_hint}", &profile.gateway_version_hint);

    if !had_runtime_placeholder && !profile.runtime_args.trim().is_empty() {
        format!("{} {}", rendered.trim_end(), profile.runtime_args)
    } else {
        rendered
    }
}

fn run_shell_command(
    command: &str,
    timeout_seconds: u64,
    cancel_flag: Option<&AtomicBool>,
) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("cmd")
            .args(["/C", command])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败: {}", e))?;

        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| format!("轮询命令状态失败: {}", e))?
            {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("读取命令输出失败: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !status.success() {
                    return Err(if stderr.is_empty() {
                        format!("退出码 {:?}", status.code())
                    } else {
                        stderr
                    });
                }
                return Ok(if stdout.is_empty() {
                    None
                } else {
                    Some(truncate_output(&stdout))
                });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
            }

            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                let _ = child.kill();
                return Err("用户已取消当前批次验证".to_string());
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut child = Command::new("sh")
            .args(["-lc", command])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败: {}", e))?;

        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| format!("轮询命令状态失败: {}", e))?
            {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("读取命令输出失败: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !status.success() {
                    return Err(if stderr.is_empty() {
                        format!("退出码 {:?}", status.code())
                    } else {
                        stderr
                    });
                }
                return Ok(if stdout.is_empty() {
                    None
                } else {
                    Some(truncate_output(&stdout))
                });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
            }

            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                let _ = child.kill();
                return Err("用户已取消当前批次验证".to_string());
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
}

fn truncate_output(raw: &str) -> String {
    const MAX_CHARS: usize = 280;
    if raw.chars().count() <= MAX_CHARS {
        return raw.to_string();
    }
    raw.chars().take(MAX_CHARS).collect::<String>() + "..."
}

fn classify_execution_category(success: bool, message: &str) -> String {
    if success {
        return "success".to_string();
    }
    let lower = message.to_ascii_lowercase();
    if message.contains("未找到绑定的 IDE 账号") || message.contains("未找到对应的 IDE 账号")
    {
        return "account_not_found".to_string();
    }
    if message.contains("账号注入失败") {
        return "inject_failed".to_string();
    }
    if message.contains("超时") {
        return "timeout".to_string();
    }
    if message.contains("用户已取消当前批次验证") {
        return "canceled".to_string();
    }
    if lower.contains("not recognized")
        || lower.contains("not found")
        || lower.contains("no such file")
        || message.contains("不是内部或外部命令")
    {
        return "command_not_found".to_string();
    }
    if lower.contains("permission denied") || message.contains("拒绝访问") {
        return "permission_denied".to_string();
    }
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
    {
        return "auth_failed".to_string();
    }
    if lower.contains("429") || lower.contains("rate limit") {
        return "rate_limited".to_string();
    }
    if message.contains("执行命令失败") {
        return "command_failed".to_string();
    }
    if message.contains("缺少账号")
        || message.contains("缺少命令模板")
        || message.contains("缺少模型")
        || message.contains("缺少 cron")
    {
        return "validation_failed".to_string();
    }
    "error_unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_task() -> WakeupTask {
        WakeupTask {
            id: "task-1".to_string(),
            name: "test".to_string(),
            enabled: true,
            account_id: "acc-1".to_string(),
            trigger_mode: "quota_reset".to_string(),
            reset_window: "primary_window".to_string(),
            window_day_policy: "all_days".to_string(),
            window_fallback_policy: "none".to_string(),
            client_version_mode: "auto".to_string(),
            client_version_fallback_mode: "auto".to_string(),
            command_template: "echo test".to_string(),
            model: "gpt-5".to_string(),
            prompt: "hi".to_string(),
            cron: "0 */6 * * *".to_string(),
            notes: None,
            timeout_seconds: 120,
            retry_failed_times: 0,
            pause_after_failures: 0,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            last_run_at: None,
            last_status: None,
            last_category: None,
            last_message: None,
            consecutive_failures: 0,
        }
    }

    #[test]
    fn client_profile_falls_back_for_unsupported_platform() {
        let mut task = sample_task();
        task.client_version_mode = "official_preview".to_string();
        task.client_version_fallback_mode = "official_legacy".to_string();
        let profile = resolve_task_client_profile(&task, "cursor");
        assert_eq!(profile.requested_mode, "official_preview");
        assert_eq!(profile.effective_mode, "official_legacy");
        assert!(profile
            .fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("已回退"));
        assert!(profile.runtime_args.contains("--legacy-auth-flow"));
    }

    #[test]
    fn render_template_appends_runtime_args_when_placeholder_missing() {
        let mut task = sample_task();
        task.command_template = "gemini -m \"{model}\" -p \"{prompt}\"".to_string();
        task.client_version_mode = "official_preview".to_string();
        let profile = resolve_task_client_profile(&task, "gemini");
        let rendered = render_command_template(&task, "demo@example.com", &profile);
        assert!(rendered.contains("gemini -m \"gpt-5\" -p \"hi\""));
        assert!(rendered.contains("--client-channel preview --enable-preview"));
    }

    #[test]
    fn render_template_resolves_gateway_placeholders() {
        let mut task = sample_task();
        task.command_template =
            "cmd --mode {gateway_mode} --transport {gateway_transport} --routing {gateway_routing}"
                .to_string();
        task.client_version_mode = "official_stable".to_string();
        let profile = resolve_task_client_profile(&task, "codex");
        let rendered = render_command_template(&task, "demo@example.com", &profile);
        assert!(rendered.contains("--mode strict"));
        assert!(rendered.contains("--transport oauth_token"));
        assert!(rendered.contains("--routing codex_official"));
    }
}
