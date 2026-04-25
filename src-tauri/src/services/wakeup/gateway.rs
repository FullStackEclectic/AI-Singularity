use super::batch::{register_cancellation_flag, unregister_cancellation_flag};
use super::execution::{
    apply_attempt_outcome_to_task, execute_wakeup_task_with_retry, notify_task_auto_paused,
    WakeupAttemptOutcome,
};
use super::{
    WakeupCategoryCount, WakeupHistoryItem, WakeupService, WakeupTask, WakeupVerificationBatchItem,
    WakeupVerificationBatchResult,
};
use crate::db::{Database, WakeupHistoryRow, WakeupRunRow};
use crate::models::{AccountStatus, IdeAccount};
use crate::services::account_health::AccountHealthService;
use crate::services::event_bus::EventBus;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

const DEFAULT_GATEWAY_CONCURRENCY: usize = 3;
const CONCURRENCY_LIMIT_KEY: &str = "wakeup_gateway_concurrency";

static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Scheduler,
    Verification,
    Manual,
    Event,
    Chain,
}

impl RunKind {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RunKind::Scheduler => "scheduler",
            RunKind::Verification => "verification",
            RunKind::Manual => "manual",
            RunKind::Event => "event",
            RunKind::Chain => "chain",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DispatchAccountInput {
    pub account_id: String,
    pub task_id: Option<String>,
    pub task_name: String,
}

#[derive(Debug, Clone)]
pub struct DispatchRequest {
    pub kind: RunKind,
    pub task_template: WakeupTask,
    pub accounts: Vec<DispatchAccountInput>,
    pub triggered_by: String,
    pub run_id: Option<String>,
    pub mutate_task: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchAccountOutcome {
    pub account_id: String,
    pub email: String,
    pub status: String,
    pub category: String,
    pub attempts: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DispatchOutcome {
    pub run_id: String,
    pub items: Vec<DispatchAccountOutcome>,
    pub canceled: bool,
    pub success_count: usize,
    pub failed_count: usize,
    pub retried_count: usize,
}

pub struct WakeupGateway;

impl WakeupGateway {
    pub fn dispatch(app: &AppHandle, mut req: DispatchRequest) -> Result<DispatchOutcome, String> {
        let db = app.state::<Database>();
        let limit = Self::concurrency_limit(&db);
        let prev = IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
        if prev >= limit {
            IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
            return Err(format!(
                "Wakeup 网关并发已达上限 {}，请稍后再试",
                limit
            ));
        }

        let outcome = Self::dispatch_inner(app, &db, &mut req);

        IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
        outcome
    }

    fn dispatch_inner(
        app: &AppHandle,
        db: &Database,
        req: &mut DispatchRequest,
    ) -> Result<DispatchOutcome, String> {
        let started_at = Utc::now();
        let started_at_str = started_at.to_rfc3339();
        let run_id = req
            .run_id
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("run-{}", uuid::Uuid::new_v4()));

        let cancel_flag = register_cancellation_flag(&run_id);

        let run_row = WakeupRunRow {
            id: run_id.clone(),
            kind: req.kind.as_db_str().to_string(),
            task_id: option_when_present(&req.task_template.id),
            triggered_by: req.triggered_by.clone(),
            started_at: started_at_str.clone(),
            finished_at: None,
            total_count: req.accounts.len() as i64,
            success_count: 0,
            failed_count: 0,
            canceled: false,
            summary_json: None,
        };
        if let Err(e) = db.create_wakeup_run(&run_row) {
            tracing::warn!("[Wakeup] 创建 run 记录失败 run_id={} err={}", run_id, e);
        }

        let accounts = db
            .get_all_ide_accounts()
            .map_err(|e| format!("读取账号列表失败: {}", e))?;
        let mut items: Vec<DispatchAccountOutcome> = Vec::with_capacity(req.accounts.len());
        let mut history_items: Vec<WakeupHistoryRow> = Vec::with_capacity(req.accounts.len());
        let mut canceled = false;
        let mut retried_count: usize = 0;

        for input in req.accounts.iter() {
            if cancel_flag.load(Ordering::Relaxed) {
                canceled = true;
                break;
            }

            let account = match accounts.iter().find(|a| a.id == input.account_id) {
                Some(a) => a.clone(),
                None => {
                    let outcome = DispatchAccountOutcome {
                        account_id: input.account_id.clone(),
                        email: "未知账号".to_string(),
                        status: "error".to_string(),
                        category: "account_not_found".to_string(),
                        attempts: 0,
                        message: "未找到对应的 IDE 账号".to_string(),
                    };
                    history_items.push(build_history_row(
                        &run_id,
                        input,
                        &outcome,
                        &req.task_template.model,
                    ));
                    items.push(outcome);
                    continue;
                }
            };

            if should_skip_for_health(&account) {
                let outcome = DispatchAccountOutcome {
                    account_id: account.id.clone(),
                    email: account.email.clone(),
                    status: "skipped".to_string(),
                    category: "skipped_forbidden".to_string(),
                    attempts: 0,
                    message: account
                        .disabled_reason
                        .clone()
                        .unwrap_or_else(|| "账号已被禁用".to_string()),
                };
                history_items.push(build_history_row(
                    &run_id,
                    input,
                    &outcome,
                    &req.task_template.model,
                ));
                items.push(outcome);
                continue;
            }

            let mut task = req.task_template.clone();
            task.account_id = account.id.clone();
            if let Some(tid) = input.task_id.as_ref() {
                task.id = tid.clone();
            }
            if !input.task_name.trim().is_empty() {
                task.name = input.task_name.clone();
            }

            let attempt_outcome = execute_wakeup_task_with_retry(
                db,
                &task,
                task.retry_failed_times as usize,
                Some(cancel_flag.as_ref()),
            );
            if attempt_outcome
                .execution
                .message
                .contains("用户已取消当前批次验证")
            {
                canceled = true;
            }
            retried_count += attempt_outcome.attempts.saturating_sub(1);
            apply_health_feedback(db, &account, &attempt_outcome);

            if req.mutate_task {
                let now_str = Utc::now().to_rfc3339();
                let mut mutable_task = task.clone();
                let auto_paused =
                    apply_attempt_outcome_to_task(&mut mutable_task, &attempt_outcome, &now_str);
                if let Err(err) = WakeupService::upsert_task(db, &mutable_task) {
                    tracing::warn!(
                        "[Wakeup] 任务状态回写失败 task_id={} err={}",
                        mutable_task.id,
                        err
                    );
                }
                if auto_paused {
                    notify_task_auto_paused(app, &mutable_task);
                }
            }

            let status = if attempt_outcome.execution.success {
                "success"
            } else {
                "error"
            }
            .to_string();
            let outcome = DispatchAccountOutcome {
                account_id: account.id.clone(),
                email: account.email.clone(),
                status,
                category: attempt_outcome.category.clone(),
                attempts: attempt_outcome.attempts,
                message: attempt_outcome.execution.message.clone(),
            };
            history_items.push(build_history_row(
                &run_id,
                input,
                &outcome,
                &req.task_template.model,
            ));
            items.push(outcome);

            if canceled {
                break;
            }
        }

        unregister_cancellation_flag(&run_id);

        let success_count = items.iter().filter(|i| i.status == "success").count();
        let failed_count = items
            .iter()
            .filter(|i| i.status == "error" || i.status == "skipped")
            .count();
        let summary_json = build_summary_json(&items);

        if !history_items.is_empty() {
            if let Err(e) = db.append_wakeup_history(&history_items) {
                tracing::warn!("[Wakeup] 追加历史失败 run_id={} err={}", run_id, e);
            }
        }
        let finished_at = Utc::now().to_rfc3339();
        if let Err(e) = db.finalize_wakeup_run(
            &run_id,
            &finished_at,
            items.len() as i64,
            success_count as i64,
            failed_count as i64,
            canceled,
            Some(summary_json.as_str()),
        ) {
            tracing::warn!("[Wakeup] 收尾 run 失败 run_id={} err={}", run_id, e);
        }

        EventBus::emit_data_changed(
            app,
            "wakeup",
            "run_finished",
            &format!("wakeup.gateway:{}", run_id),
        );

        // 链式触发：在异步任务里 spawn 派发下游 chain task，避免同步递归
        if !req.task_template.id.trim().is_empty() {
            super::listener::resolve_chain_after_dispatch(
                app,
                &req.task_template.id,
                success_count > 0,
            );
        }

        Ok(DispatchOutcome {
            run_id,
            items,
            canceled,
            success_count,
            failed_count,
            retried_count,
        })
    }

    pub fn cancel(run_id: &str) -> bool {
        WakeupService::cancel_verification_run(run_id).unwrap_or(false)
    }

    pub fn current_in_flight() -> usize {
        IN_FLIGHT.load(Ordering::SeqCst)
    }

    pub fn concurrency_limit(db: &Database) -> usize {
        static CACHE: OnceLock<()> = OnceLock::new();
        let _ = CACHE; // reserved for future caching strategy

        match db.get_account_setting(CONCURRENCY_LIMIT_KEY) {
            Ok(Some(raw)) => raw
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|n| *n >= 1 && *n <= 32)
                .unwrap_or(DEFAULT_GATEWAY_CONCURRENCY),
            _ => DEFAULT_GATEWAY_CONCURRENCY,
        }
    }
}

fn option_when_present(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn should_skip_for_health(account: &IdeAccount) -> bool {
    matches!(account.status, AccountStatus::Forbidden)
        && AccountHealthService::is_invalid_grant_disabled(account)
}

fn apply_health_feedback(db: &Database, account: &IdeAccount, outcome: &WakeupAttemptOutcome) {
    if outcome.execution.success {
        AccountHealthService::try_clear_invalid_grant(db, account);
        return;
    }
    if outcome.category == "auth_failed"
        && AccountHealthService::looks_like_invalid_grant(&outcome.execution.message)
    {
        AccountHealthService::mark_invalid_grant(db, &account.id, &outcome.execution.message);
    }
}

fn build_history_row(
    run_id: &str,
    input: &DispatchAccountInput,
    outcome: &DispatchAccountOutcome,
    model: &str,
) -> WakeupHistoryRow {
    WakeupHistoryRow {
        id: format!("history-{}", uuid::Uuid::new_v4()),
        run_id: run_id.to_string(),
        task_id: input.task_id.clone(),
        task_name: if input.task_name.trim().is_empty() {
            "未命名任务".to_string()
        } else {
            input.task_name.clone()
        },
        account_id: outcome.account_id.clone(),
        model: model.to_string(),
        status: outcome.status.clone(),
        category: outcome.category.clone(),
        message: Some(if outcome.attempts > 1 {
            format!("（尝试 {} 次）{}", outcome.attempts, outcome.message)
        } else {
            outcome.message.clone()
        }),
        attempts: outcome.attempts as i64,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn build_summary_json(items: &[DispatchAccountOutcome]) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in items {
        *counts.entry(item.category.clone()).or_insert(0) += 1;
    }
    serde_json::json!({
        "categories": counts,
    })
    .to_string()
}

/// Adapter: convert a Gateway dispatch result into the legacy
/// `WakeupVerificationBatchResult` shape that the existing frontend expects.
pub fn dispatch_outcome_to_batch_result(
    outcome: DispatchOutcome,
) -> WakeupVerificationBatchResult {
    let mut category_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut items: Vec<WakeupVerificationBatchItem> = Vec::with_capacity(outcome.items.len());
    for item in outcome.items.iter() {
        *category_map.entry(item.category.clone()).or_insert(0) += 1;
        items.push(WakeupVerificationBatchItem {
            account_id: item.account_id.clone(),
            email: item.email.clone(),
            status: item.status.clone(),
            category: item.category.clone(),
            attempts: item.attempts,
            message: item.message.clone(),
        });
    }
    let category_counts = category_map
        .into_iter()
        .map(|(category, count)| WakeupCategoryCount { category, count })
        .collect::<Vec<_>>();
    WakeupVerificationBatchResult {
        executed_count: items.len(),
        success_count: outcome.success_count,
        failed_count: outcome.failed_count,
        retried_count: outcome.retried_count,
        canceled: outcome.canceled,
        category_counts,
        items,
    }
}

/// Adapter: also build a legacy `WakeupHistoryItem` list from a dispatch
/// outcome so callers that still expose the legacy JSON-shaped event stream
/// (e.g. `wakeup_add_history` consumers) keep working.
pub fn dispatch_outcome_to_history_items(
    outcome: &DispatchOutcome,
    model: &str,
    task_name: &str,
    task_id: Option<&str>,
) -> Vec<WakeupHistoryItem> {
    let now = Utc::now().to_rfc3339();
    outcome
        .items
        .iter()
        .map(|item| WakeupHistoryItem {
            id: format!("history-{}", uuid::Uuid::new_v4()),
            run_id: Some(outcome.run_id.clone()),
            task_id: task_id.map(|v| v.to_string()),
            task_name: task_name.to_string(),
            account_id: item.account_id.clone(),
            model: model.to_string(),
            status: item.status.clone(),
            category: item.category.clone(),
            message: Some(if item.attempts > 1 {
                format!("（尝试 {} 次）{}", item.attempts, item.message)
            } else {
                item.message.clone()
            }),
            created_at: now.clone(),
        })
        .collect()
}
