use crate::db::Database;
use crate::services::event_bus::EventBus;
use crate::services::ide_injector::IdeInjector;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const WAKEUP_STATE_FILE: &str = "wakeup_state.json";
const WAKEUP_HISTORY_FILE: &str = "wakeup_history.json";
const MAX_HISTORY_ITEMS: usize = 200;
const SCHEDULER_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupTask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub account_id: String,
    #[serde(default)]
    pub command_template: String,
    pub model: String,
    pub prompt: String,
    pub cron: String,
    pub notes: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WakeupState {
    pub enabled: bool,
    pub tasks: Vec<WakeupTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupHistoryItem {
    pub id: String,
    pub task_id: Option<String>,
    pub task_name: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchItem {
    pub account_id: String,
    pub email: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupVerificationBatchResult {
    pub executed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub items: Vec<WakeupVerificationBatchItem>,
}

pub struct WakeupService;
static WAKEUP_SCHEDULER_STARTED: OnceLock<()> = OnceLock::new();

fn default_timeout_seconds() -> u64 {
    120
}

impl WakeupService {
    pub fn load_state(app_data_dir: &Path) -> Result<WakeupState, String> {
        let path = state_path(app_data_dir);
        if !path.exists() {
            return Ok(WakeupState::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 状态失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(WakeupState::default());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 状态失败: {}", e))
    }

    pub fn save_state(app_data_dir: &Path, mut state: WakeupState) -> Result<WakeupState, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let now = Utc::now().to_rfc3339();
        for task in &mut state.tasks {
            if task.created_at.trim().is_empty() {
                task.created_at = now.clone();
            }
            task.updated_at = now.clone();
        }
        let content =
            serde_json::to_string_pretty(&state).map_err(|e| format!("序列化 Wakeup 状态失败: {}", e))?;
        fs::write(state_path(app_data_dir), content).map_err(|e| format!("写入 Wakeup 状态失败: {}", e))?;
        Ok(state)
    }

    pub fn load_history(app_data_dir: &Path) -> Result<Vec<WakeupHistoryItem>, String> {
        let path = history_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("读取 Wakeup 历史失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&raw).map_err(|e| format!("解析 Wakeup 历史失败: {}", e))
    }

    pub fn add_history_items(
        app_data_dir: &Path,
        items: Vec<WakeupHistoryItem>,
    ) -> Result<Vec<WakeupHistoryItem>, String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let mut current = Self::load_history(app_data_dir)?;
        current.extend(items);
        current.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        current.truncate(MAX_HISTORY_ITEMS);
        let content = serde_json::to_string_pretty(&current)
            .map_err(|e| format!("序列化 Wakeup 历史失败: {}", e))?;
        fs::write(history_path(app_data_dir), content).map_err(|e| format!("写入 Wakeup 历史失败: {}", e))?;
        Ok(current)
    }

    pub fn clear_history(app_data_dir: &Path) -> Result<(), String> {
        let path = history_path(app_data_dir);
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("清空 Wakeup 历史失败: {}", e))?;
        }
        Ok(())
    }

    pub fn run_task_now(app: &AppHandle, task_id: &str) -> Result<WakeupTask, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用目录失败: {}", e))?;
        let mut state = Self::load_state(&app_data_dir)?;
        let db = app.state::<Database>();
        let now = Utc::now().to_rfc3339();

        let task = state
            .tasks
            .iter_mut()
            .find(|item| item.id == task_id)
            .ok_or_else(|| "未找到对应的 Wakeup 任务".to_string())?;

        if task.account_id.trim().is_empty()
            || task.model.trim().is_empty()
            || task.command_template.trim().is_empty()
        {
            return Err("任务缺少账号、模型或命令模板，无法立即执行".to_string());
        }

        let execution = execute_wakeup_task(&db, task);
        task.last_run_at = Some(now.clone());
        task.updated_at = now.clone();
        task.last_status = Some(if execution.success { "success" } else { "error" }.to_string());
        task.last_message = Some(execution.message.clone());
        let saved = Self::save_state(&app_data_dir, state)?;
        let updated_task = saved
            .tasks
            .into_iter()
            .find(|item| item.id == task_id)
            .ok_or_else(|| "任务执行后未能重新读取状态".to_string())?;

        let _ = Self::add_history_items(
            &app_data_dir,
            vec![WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                task_id: Some(updated_task.id.clone()),
                task_name: if updated_task.name.trim().is_empty() {
                    "未命名任务".to_string()
                } else {
                    updated_task.name.clone()
                },
                account_id: updated_task.account_id.clone(),
                model: updated_task.model.clone(),
                status: updated_task
                    .last_status
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                message: updated_task.last_message.clone(),
                created_at: now,
            }],
        )?;

        EventBus::emit_data_changed(app, "wakeup", "run_task_now", "wakeup.run_task_now");
        Ok(updated_task)
    }

    pub fn run_verification_batch(
        app: &AppHandle,
        account_ids: Vec<String>,
        model: &str,
        prompt: &str,
        command_template: &str,
        timeout_seconds: u64,
    ) -> Result<WakeupVerificationBatchResult, String> {
        let db = app.state::<Database>();
        let accounts = db.get_all_ide_accounts().map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        let mut history_items = Vec::new();
        let now = Utc::now().to_rfc3339();

        for account_id in account_ids.into_iter().filter(|id| !id.trim().is_empty()) {
            let Some(account) = accounts.iter().find(|item| item.id == account_id) else {
                items.push(WakeupVerificationBatchItem {
                    account_id: account_id.clone(),
                    email: "未知账号".to_string(),
                    status: "error".to_string(),
                    message: "未找到对应的 IDE 账号".to_string(),
                });
                continue;
            };

            let task = WakeupTask {
                id: format!("verification-{}", account.id),
                name: "批次验证".to_string(),
                enabled: true,
                account_id: account.id.clone(),
                command_template: command_template.to_string(),
                model: model.to_string(),
                prompt: prompt.to_string(),
                cron: String::new(),
                notes: None,
                timeout_seconds,
                created_at: now.clone(),
                updated_at: now.clone(),
                last_run_at: None,
                last_status: None,
                last_message: None,
            };

            let execution = execute_wakeup_task(&db, &task);
            let status = if execution.success { "success" } else { "error" }.to_string();

            items.push(WakeupVerificationBatchItem {
                account_id: account.id.clone(),
                email: account.email.clone(),
                status: status.clone(),
                message: execution.message.clone(),
            });

            history_items.push(WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                task_id: None,
                task_name: "批次验证".to_string(),
                account_id: account.id.clone(),
                model: model.to_string(),
                status,
                message: Some(execution.message),
                created_at: Utc::now().to_rfc3339(),
            });
        }

        if !history_items.is_empty() {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用目录失败: {}", e))?;
            let _ = Self::add_history_items(&app_data_dir, history_items)?;
            EventBus::emit_data_changed(app, "wakeup", "verification_batch", "wakeup.verification_batch");
        }

        let success_count = items.iter().filter(|item| item.status == "success").count();
        let failed_count = items.len().saturating_sub(success_count);

        Ok(WakeupVerificationBatchResult {
            executed_count: items.len(),
            success_count,
            failed_count,
            items,
        })
    }

    pub fn ensure_scheduler_started(app: AppHandle, app_data_dir: PathBuf) {
        if WAKEUP_SCHEDULER_STARTED.set(()).is_err() {
            return;
        }

        tauri::async_runtime::spawn(async move {
            let app_handle = app.clone();
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(SCHEDULER_INTERVAL_SECS)).await;
                if let Err(err) = Self::run_scheduler_tick(&app_handle, &app_data_dir) {
                    tracing::warn!("[Wakeup] 调度器执行失败: {}", err);
                }
            }
        });
    }

    fn run_scheduler_tick(app: &AppHandle, app_data_dir: &Path) -> Result<(), String> {
        let mut state = Self::load_state(app_data_dir)?;
        if !state.enabled {
            return Ok(());
        }
        let db = app.state::<Database>();

        let now = Utc::now();
        let window_start = now - ChronoDuration::seconds((SCHEDULER_INTERVAL_SECS as i64) + 35);
        let mut history_items = Vec::new();
        let mut changed = false;

        for task in &mut state.tasks {
            if !task.enabled {
                continue;
            }
            if task.account_id.trim().is_empty()
                || task.model.trim().is_empty()
                || task.cron.trim().is_empty()
                || task.command_template.trim().is_empty()
            {
                continue;
            }

            let Some(scheduled_for) = next_due_in_window(&task.cron, window_start, now) else {
                continue;
            };

            let already_ran = task
                .last_run_at
                .as_deref()
                .and_then(parse_rfc3339_utc)
                .is_some_and(|last| last >= scheduled_for);
            if already_ran {
                continue;
            }

            let execution = execute_wakeup_task(&db, task);
            task.last_run_at = Some(now.to_rfc3339());
            task.updated_at = now.to_rfc3339();
            task.last_status = Some(if execution.success { "success" } else { "error" }.to_string());
            task.last_message = Some(execution.message.clone());
            changed = true;

            history_items.push(WakeupHistoryItem {
                id: format!("history-{}", uuid::Uuid::new_v4()),
                task_id: Some(task.id.clone()),
                task_name: if task.name.trim().is_empty() {
                    "未命名任务".to_string()
                } else {
                    task.name.clone()
                },
                account_id: task.account_id.clone(),
                model: task.model.clone(),
                status: if execution.success {
                    "success".to_string()
                } else {
                    "error".to_string()
                },
                message: Some(format!(
                    "调度器命中了 cron 表达式（{}）。{}",
                    task.cron, execution.message
                )),
                created_at: now.to_rfc3339(),
            });
        }

        if changed {
            let saved = Self::save_state(app_data_dir, state)?;
            let _ = saved;
            if !history_items.is_empty() {
                let _ = Self::add_history_items(app_data_dir, history_items)?;
            }
            EventBus::emit_data_changed(app, "wakeup", "scheduler_tick", "wakeup.scheduler");
        }

        Ok(())
    }
}

fn parse_rfc3339_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn next_due_in_window(
    cron_expr: &str,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let schedule = Schedule::from_str(cron_expr).ok()?;
    let candidate = schedule.after(&window_start).next()?;
    if candidate <= now {
        Some(candidate)
    } else {
        None
    }
}

struct WakeupExecutionResult {
    success: bool,
    message: String,
}

fn execute_wakeup_task(db: &Database, task: &WakeupTask) -> WakeupExecutionResult {
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

    let command = render_command_template(task, &account.email);
    match run_shell_command(&command, task.timeout_seconds) {
        Ok(output) => WakeupExecutionResult {
            success: true,
            message: format!(
                "已执行命令：{}{}",
                command,
                output
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("；输出：{}", value))
                    .unwrap_or_default()
            ),
        },
        Err(err) => WakeupExecutionResult {
            success: false,
            message: format!("执行命令失败：{}；命令：{}", err, command),
        },
    }
}

fn render_command_template(task: &WakeupTask, email: &str) -> String {
    task.command_template
        .replace("{model}", &task.model)
        .replace("{prompt}", &task.prompt)
        .replace("{account_id}", &task.account_id)
        .replace("{email}", email)
}

fn run_shell_command(command: &str, timeout_seconds: u64) -> Result<Option<String>, String> {
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
            if let Some(status) = child.try_wait().map_err(|e| format!("轮询命令状态失败: {}", e))? {
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
                return Ok(if stdout.is_empty() { None } else { Some(truncate_output(&stdout)) });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
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
            if let Some(status) = child.try_wait().map_err(|e| format!("轮询命令状态失败: {}", e))? {
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
                return Ok(if stdout.is_empty() { None } else { Some(truncate_output(&stdout)) });
            }

            if started.elapsed().as_secs() >= timeout_seconds {
                let _ = child.kill();
                return Err(format!("命令执行超时（{} 秒）", timeout_seconds));
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

fn state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_STATE_FILE)
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(WAKEUP_HISTORY_FILE)
}
