use super::Database;
use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupTaskRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub account_id: String,
    pub trigger_mode: String,
    pub config_json: String,
    pub model: String,
    pub prompt: Option<String>,
    pub command_template: String,
    pub notes: Option<String>,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_category: Option<String>,
    pub last_message: Option<String>,
    pub consecutive_failures: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupRunRow {
    pub id: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub triggered_by: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub total_count: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub canceled: bool,
    pub summary_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupHistoryRow {
    pub id: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub task_name: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    pub category: String,
    pub message: Option<String>,
    pub attempts: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupCategorySummary {
    pub category: String,
    pub total: i64,
    pub success: i64,
}

fn map_task_row(row: &rusqlite::Row<'_>) -> SqlResult<WakeupTaskRow> {
    let enabled_int: i64 = row.get(2)?;
    let consecutive: i64 = row.get(14)?;
    Ok(WakeupTaskRow {
        id: row.get(0)?,
        name: row.get(1)?,
        enabled: enabled_int != 0,
        account_id: row.get(3)?,
        trigger_mode: row.get(4)?,
        config_json: row.get(5)?,
        model: row.get(6)?,
        prompt: row.get(7)?,
        command_template: row.get(8)?,
        notes: row.get(9)?,
        last_run_at: row.get(10)?,
        last_status: row.get(11)?,
        last_category: row.get(12)?,
        last_message: row.get(13)?,
        consecutive_failures: consecutive,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn map_run_row(row: &rusqlite::Row<'_>) -> SqlResult<WakeupRunRow> {
    let canceled_int: i64 = row.get(9)?;
    Ok(WakeupRunRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        task_id: row.get(2)?,
        triggered_by: row.get(3)?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
        total_count: row.get(6)?,
        success_count: row.get(7)?,
        failed_count: row.get(8)?,
        canceled: canceled_int != 0,
        summary_json: row.get(10)?,
    })
}

fn map_history_row(row: &rusqlite::Row<'_>) -> SqlResult<WakeupHistoryRow> {
    Ok(WakeupHistoryRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        task_id: row.get(2)?,
        task_name: row.get(3)?,
        account_id: row.get(4)?,
        model: row.get(5)?,
        status: row.get(6)?,
        category: row.get(7)?,
        message: row.get(8)?,
        attempts: row.get(9)?,
        created_at: row.get(10)?,
    })
}

const TASK_COLUMNS: &str = "id, name, enabled, account_id, trigger_mode, config_json, model, prompt, command_template, notes, last_run_at, last_status, last_category, last_message, consecutive_failures, created_at, updated_at";

const RUN_COLUMNS: &str = "id, kind, task_id, triggered_by, started_at, finished_at, total_count, success_count, failed_count, canceled, summary_json";

const HISTORY_COLUMNS: &str = "id, run_id, task_id, task_name, account_id, model, status, category, message, attempts, created_at";

impl Database {
    pub fn list_wakeup_tasks(&self) -> SqlResult<Vec<WakeupTaskRow>> {
        self.query_rows(
            &format!(
                "SELECT {} FROM wakeup_tasks ORDER BY created_at ASC",
                TASK_COLUMNS
            ),
            &[],
            map_task_row,
        )
    }

    pub fn get_wakeup_task(&self, id: &str) -> SqlResult<Option<WakeupTaskRow>> {
        Ok(self
            .query_row(
                &format!("SELECT {} FROM wakeup_tasks WHERE id = ?1", TASK_COLUMNS),
                &[&id],
                map_task_row,
            )
            .ok())
    }

    pub fn upsert_wakeup_task(&self, task: &WakeupTaskRow) -> SqlResult<usize> {
        self.execute(
            "INSERT INTO wakeup_tasks (id, name, enabled, account_id, trigger_mode, config_json, model, prompt, command_template, notes, last_run_at, last_status, last_category, last_message, consecutive_failures, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                enabled = excluded.enabled,
                account_id = excluded.account_id,
                trigger_mode = excluded.trigger_mode,
                config_json = excluded.config_json,
                model = excluded.model,
                prompt = excluded.prompt,
                command_template = excluded.command_template,
                notes = excluded.notes,
                last_run_at = excluded.last_run_at,
                last_status = excluded.last_status,
                last_category = excluded.last_category,
                last_message = excluded.last_message,
                consecutive_failures = excluded.consecutive_failures,
                updated_at = excluded.updated_at",
            rusqlite::params![
                task.id,
                task.name,
                if task.enabled { 1_i64 } else { 0_i64 },
                task.account_id,
                task.trigger_mode,
                task.config_json,
                task.model,
                task.prompt,
                task.command_template,
                task.notes,
                task.last_run_at,
                task.last_status,
                task.last_category,
                task.last_message,
                task.consecutive_failures,
                task.created_at,
                task.updated_at,
            ],
        )
    }

    pub fn delete_wakeup_task(&self, id: &str) -> SqlResult<usize> {
        self.execute("DELETE FROM wakeup_tasks WHERE id = ?1", &[&id])
    }

    pub fn replace_wakeup_tasks(&self, tasks: &[WakeupTaskRow]) -> SqlResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let keep_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        if keep_ids.is_empty() {
            tx.execute("DELETE FROM wakeup_tasks", [])?;
        } else {
            let placeholders = std::iter::repeat("?")
                .take(keep_ids.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "DELETE FROM wakeup_tasks WHERE id NOT IN ({})",
                placeholders
            );
            let params: Vec<&dyn rusqlite::ToSql> = keep_ids
                .iter()
                .map(|id| id as &dyn rusqlite::ToSql)
                .collect();
            tx.execute(&sql, params.as_slice())?;
        }
        for task in tasks {
            tx.execute(
                "INSERT INTO wakeup_tasks (id, name, enabled, account_id, trigger_mode, config_json, model, prompt, command_template, notes, last_run_at, last_status, last_category, last_message, consecutive_failures, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    enabled = excluded.enabled,
                    account_id = excluded.account_id,
                    trigger_mode = excluded.trigger_mode,
                    config_json = excluded.config_json,
                    model = excluded.model,
                    prompt = excluded.prompt,
                    command_template = excluded.command_template,
                    notes = excluded.notes,
                    last_run_at = excluded.last_run_at,
                    last_status = excluded.last_status,
                    last_category = excluded.last_category,
                    last_message = excluded.last_message,
                    consecutive_failures = excluded.consecutive_failures,
                    updated_at = excluded.updated_at",
                rusqlite::params![
                    task.id,
                    task.name,
                    if task.enabled { 1_i64 } else { 0_i64 },
                    task.account_id,
                    task.trigger_mode,
                    task.config_json,
                    task.model,
                    task.prompt,
                    task.command_template,
                    task.notes,
                    task.last_run_at,
                    task.last_status,
                    task.last_category,
                    task.last_message,
                    task.consecutive_failures,
                    task.created_at,
                    task.updated_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn create_wakeup_run(&self, run: &WakeupRunRow) -> SqlResult<usize> {
        self.execute(
            "INSERT INTO wakeup_runs (id, kind, task_id, triggered_by, started_at, finished_at, total_count, success_count, failed_count, canceled, summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                run.id,
                run.kind,
                run.task_id,
                run.triggered_by,
                run.started_at,
                run.finished_at,
                run.total_count,
                run.success_count,
                run.failed_count,
                if run.canceled { 1_i64 } else { 0_i64 },
                run.summary_json,
            ],
        )
    }

    pub fn finalize_wakeup_run(
        &self,
        run_id: &str,
        finished_at: &str,
        total: i64,
        success: i64,
        failed: i64,
        canceled: bool,
        summary_json: Option<&str>,
    ) -> SqlResult<usize> {
        self.execute(
            "UPDATE wakeup_runs SET finished_at = ?1, total_count = ?2, success_count = ?3, failed_count = ?4, canceled = ?5, summary_json = ?6 WHERE id = ?7",
            rusqlite::params![
                finished_at,
                total,
                success,
                failed,
                if canceled { 1_i64 } else { 0_i64 },
                summary_json,
                run_id,
            ],
        )
    }

    pub fn list_wakeup_runs(
        &self,
        kind_filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> SqlResult<Vec<WakeupRunRow>> {
        let limit_i: i64 = limit as i64;
        let offset_i: i64 = offset as i64;
        if let Some(kind) = kind_filter {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_runs WHERE kind = ?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3",
                    RUN_COLUMNS
                ),
                &[&kind, &limit_i, &offset_i],
                map_run_row,
            )
        } else {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_runs ORDER BY started_at DESC LIMIT ?1 OFFSET ?2",
                    RUN_COLUMNS
                ),
                &[&limit_i, &offset_i],
                map_run_row,
            )
        }
    }

    pub fn count_wakeup_runs(&self, kind_filter: Option<&str>) -> SqlResult<i64> {
        if let Some(kind) = kind_filter {
            self.query_scalar(
                "SELECT COUNT(*) FROM wakeup_runs WHERE kind = ?1",
                &[&kind],
            )
        } else {
            self.query_scalar("SELECT COUNT(*) FROM wakeup_runs", &[])
        }
    }

    pub fn append_wakeup_history(&self, items: &[WakeupHistoryRow]) -> SqlResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for item in items {
            tx.execute(
                "INSERT OR REPLACE INTO wakeup_history (id, run_id, task_id, task_name, account_id, model, status, category, message, attempts, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    item.id,
                    item.run_id,
                    item.task_id,
                    item.task_name,
                    item.account_id,
                    item.model,
                    item.status,
                    item.category,
                    item.message,
                    item.attempts,
                    item.created_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_wakeup_history(
        &self,
        run_id: Option<&str>,
        limit: usize,
    ) -> SqlResult<Vec<WakeupHistoryRow>> {
        let limit_i: i64 = limit as i64;
        if let Some(rid) = run_id {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_history WHERE run_id = ?1 ORDER BY created_at DESC LIMIT ?2",
                    HISTORY_COLUMNS
                ),
                &[&rid, &limit_i],
                map_history_row,
            )
        } else {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_history ORDER BY created_at DESC LIMIT ?1",
                    HISTORY_COLUMNS
                ),
                &[&limit_i],
                map_history_row,
            )
        }
    }

    pub fn list_wakeup_history_paginated(
        &self,
        run_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> SqlResult<Vec<WakeupHistoryRow>> {
        let limit_i: i64 = limit as i64;
        let offset_i: i64 = offset as i64;
        if let Some(rid) = run_id {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_history WHERE run_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                    HISTORY_COLUMNS
                ),
                &[&rid, &limit_i, &offset_i],
                map_history_row,
            )
        } else {
            self.query_rows(
                &format!(
                    "SELECT {} FROM wakeup_history ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                    HISTORY_COLUMNS
                ),
                &[&limit_i, &offset_i],
                map_history_row,
            )
        }
    }

    pub fn clear_wakeup_history(&self) -> SqlResult<usize> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let removed = tx.execute("DELETE FROM wakeup_history", [])?;
        tx.execute("DELETE FROM wakeup_runs", [])?;
        tx.commit()?;
        Ok(removed)
    }

    pub fn list_wakeup_summary_24h(&self) -> SqlResult<Vec<WakeupCategorySummary>> {
        self.query_rows(
            "SELECT category,
                    COUNT(*) AS total,
                    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success
             FROM wakeup_history
             WHERE created_at >= datetime('now', '-1 day')
             GROUP BY category
             ORDER BY total DESC",
            &[],
            |row| {
                Ok(WakeupCategorySummary {
                    category: row.get(0)?,
                    total: row.get(1)?,
                    success: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                })
            },
        )
    }
}
