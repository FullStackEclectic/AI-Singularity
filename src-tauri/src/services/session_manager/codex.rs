use super::{
    ChatSession, CodexInstanceRef, CodexSessionRepairSummary, CodexThreadSyncItem,
    CodexThreadSyncSummary, SessionManager, ThreadRowData, ThreadSnapshot,
};
use crate::services::codex_instance_store::CodexInstanceStore;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags, Transaction};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;

impl SessionManager {
    pub fn repair_codex_session_index() -> Result<CodexSessionRepairSummary, String> {
        let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
        let instances = Self::collect_codex_instances(&home_dir)?;

        let mut total_threads = 0usize;
        let mut total_added = 0usize;
        let mut total_removed = 0usize;

        for instance in &instances {
            let (threads, added, removed) =
                Self::repair_codex_session_index_for_instance(instance)?;
            total_threads += threads;
            total_added += added;
            total_removed += removed;
        }

        Ok(CodexSessionRepairSummary {
            thread_count: total_threads,
            added_index_entries: total_added,
            removed_stale_entries: total_removed,
            message: if total_added == 0 && total_removed == 0 {
                "所有 Codex 实例的会话索引已经是最新状态".to_string()
            } else {
                format!(
                    "Codex 会话索引已修复：新增 {} 条，清理 {} 条失效索引",
                    total_added, total_removed
                )
            },
        })
    }

    pub fn sync_codex_threads_across_instances() -> Result<CodexThreadSyncSummary, String> {
        let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
        let instances = Self::collect_codex_instances(&home_dir)?;
        if instances.len() < 2 {
            return Err("至少需要两个 Codex 实例才能同步线程".to_string());
        }

        let mut thread_universe = HashMap::<String, ThreadSnapshot>::new();
        let mut existing_ids_by_instance = HashMap::<String, HashSet<String>>::new();

        for instance in &instances {
            let snapshots = Self::load_codex_thread_snapshots(instance)?;
            let ids = snapshots
                .iter()
                .map(|item| item.id.clone())
                .collect::<HashSet<_>>();
            for snapshot in snapshots {
                thread_universe.entry(snapshot.id.clone()).or_insert(snapshot);
            }
            existing_ids_by_instance.insert(instance.id.clone(), ids);
        }

        let mut universe_ids = thread_universe.keys().cloned().collect::<Vec<_>>();
        universe_ids.sort();

        let mut items = Vec::with_capacity(instances.len());
        let mut backup_dirs = Vec::new();
        let mut mutated_instance_count = 0usize;
        let mut total_synced_thread_count = 0usize;

        for instance in &instances {
            let existing_ids = existing_ids_by_instance
                .get(&instance.id)
                .cloned()
                .unwrap_or_default();
            let missing_snapshots = universe_ids
                .iter()
                .filter(|id| !existing_ids.contains(*id))
                .filter_map(|id| thread_universe.get(id).cloned())
                .collect::<Vec<_>>();

            if missing_snapshots.is_empty() {
                items.push(CodexThreadSyncItem {
                    instance_id: instance.id.clone(),
                    instance_name: instance.name.clone(),
                    added_thread_count: 0,
                    backup_dir: None,
                });
                continue;
            }

            let backup_dir =
                Self::sync_missing_codex_threads_to_instance(instance, &missing_snapshots)?;
            let backup_dir_string = backup_dir.to_string_lossy().to_string();
            backup_dirs.push(backup_dir_string.clone());
            mutated_instance_count += 1;
            total_synced_thread_count += missing_snapshots.len();

            items.push(CodexThreadSyncItem {
                instance_id: instance.id.clone(),
                instance_name: instance.name.clone(),
                added_thread_count: missing_snapshots.len(),
                backup_dir: Some(backup_dir_string),
            });
        }

        Ok(CodexThreadSyncSummary {
            instance_count: instances.len(),
            thread_universe_count: thread_universe.len(),
            mutated_instance_count,
            total_synced_thread_count,
            items,
            backup_dirs,
            message: if total_synced_thread_count == 0 {
                "所有 Codex 实例已是最新，无需同步线程".to_string()
            } else {
                format!(
                    "已为 {} 个实例补齐 {} 条线程",
                    mutated_instance_count, total_synced_thread_count
                )
            },
        })
    }

    pub(super) fn collect_codex_sessions(home_dir: &PathBuf, sessions: &mut Vec<ChatSession>) {
        let Ok(instances) = Self::collect_codex_instances(home_dir) else {
            return;
        };

        for instance in instances {
            if let Ok(instance_sessions) = Self::load_codex_sessions_for_instance(&instance) {
                sessions.extend(instance_sessions);
            }
        }
    }

    pub(super) fn collect_codex_instances(
        home_dir: &PathBuf,
    ) -> Result<Vec<CodexInstanceRef>, String> {
        let mut instances = vec![CodexInstanceRef {
            id: "__default__".to_string(),
            name: "默认实例".to_string(),
            data_dir: home_dir.join(".codex"),
        }];

        for item in CodexInstanceStore::list_instances()? {
            instances.push(CodexInstanceRef {
                id: item.id,
                name: item.name,
                data_dir: PathBuf::from(item.user_data_dir),
            });
        }

        Ok(instances)
    }

    fn load_codex_sessions_for_instance(
        instance: &CodexInstanceRef,
    ) -> Result<Vec<ChatSession>, String> {
        let snapshots = Self::load_codex_thread_snapshots(instance)?;
        Ok(snapshots
            .into_iter()
            .map(|snapshot| ChatSession {
                id: snapshot.id,
                title: format!("Codex // {}", snapshot.title),
                created_at: snapshot
                    .row_data
                    .get_i64("created_at")
                    .unwrap_or_default()
                    .max(0) as u64,
                updated_at: snapshot.updated_at.unwrap_or_default().max(0) as u64,
                messages_count: fs::read_to_string(&snapshot.rollout_path)
                    .map(|content| content.lines().count())
                    .unwrap_or(0),
                filepath: snapshot.rollout_path.to_string_lossy().into_owned(),
                tool_type: Some("Codex".to_string()),
                cwd: Some(snapshot.cwd),
                instance_id: Some(instance.id.clone()),
                instance_name: Some(instance.name.clone()),
                source_kind: Some("transcript".to_string()),
                has_tool_calls: false,
                has_log_events: false,
                latest_tool_name: None,
                latest_tool_status: None,
            })
            .collect())
    }

    fn normalize_codex_cwd(cwd: &str) -> String {
        cwd.strip_prefix(r"\\?\").unwrap_or(cwd).to_string()
    }

    fn load_codex_thread_snapshots(
        instance: &CodexInstanceRef,
    ) -> Result<Vec<ThreadSnapshot>, String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        if !db_path.exists() {
            return Ok(vec![]);
        }

        let conn = Self::open_readonly_connection(&db_path)?;
        let columns = Self::read_thread_columns(&conn)?;
        let select_columns = columns
            .iter()
            .map(|column| Self::quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!("SELECT {} FROM threads WHERE archived = 0", select_columns);
        let mut statement = conn
            .prepare(&query)
            .map_err(|e| format!("读取 Codex 线程失败 ({}): {}", instance.name, e))?;
        let mut rows = statement
            .query([])
            .map_err(|e| format!("查询 Codex 线程失败 ({}): {}", instance.name, e))?;
        let session_index_map =
            Self::read_codex_session_index_map(&instance.data_dir.join("session_index.jsonl"));

        let mut snapshots = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("迭代 Codex 线程失败 ({}): {}", instance.name, e))?
        {
            let mut values = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                values.push(row.get::<usize, Value>(index).map_err(|e| {
                    format!("解析 Codex 线程记录失败 ({}): {}", instance.name, e)
                })?);
            }

            let row_data = ThreadRowData {
                columns: columns.clone(),
                values,
            };
            let id = row_data
                .get_text("id")
                .ok_or_else(|| format!("Codex 线程缺少 id 字段 ({})", instance.name))?;
            let rollout_path = row_data.get_text("rollout_path").ok_or_else(|| {
                format!("Codex 线程 {} 缺少 rollout_path ({})", id, instance.name)
            })?;
            let cwd = row_data
                .get_text("cwd")
                .map(|value| Self::normalize_codex_cwd(&value))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "未知工作区".to_string());
            let title =
                Self::sanitize_session_title(row_data.get_text("title").unwrap_or_default())
                    .or_else(|| {
                        Self::sanitize_session_title(
                            row_data.get_text("first_user_message").unwrap_or_default(),
                        )
                    })
                    .or_else(|| {
                        session_index_map
                            .get(&id)
                            .and_then(|value| value.get("thread_name"))
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_string())
                            .and_then(Self::sanitize_session_title)
                    })
                    .or_else(|| {
                        PathBuf::from(&cwd)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| id.clone());
            let updated_at = row_data.get_i64("updated_at");
            let session_index_entry = session_index_map.get(&id).cloned().unwrap_or_else(|| {
                serde_json::json!({
                    "id": id,
                    "thread_name": title,
                    "updated_at": Self::format_index_timestamp(updated_at.unwrap_or_default()),
                })
            });

            snapshots.push(ThreadSnapshot {
                id,
                title,
                cwd,
                updated_at,
                rollout_path: PathBuf::from(rollout_path),
                row_data,
                session_index_entry,
                source_root: instance.data_dir.clone(),
            });
        }

        Ok(snapshots)
    }

    fn open_readonly_connection(db_path: &PathBuf) -> Result<Connection, String> {
        Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("打开只读数据库失败 ({}): {}", db_path.display(), e))
    }

    fn read_thread_columns(connection: &Connection) -> Result<Vec<String>, String> {
        let mut statement = connection
            .prepare("PRAGMA table_info(threads)")
            .map_err(|e| format!("读取 threads 表结构失败: {}", e))?;
        let mut rows = statement
            .query([])
            .map_err(|e| format!("查询 threads 表结构失败: {}", e))?;
        let mut columns = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("解析 threads 表结构失败: {}", e))?
        {
            columns.push(
                row.get::<usize, String>(1)
                    .map_err(|e| format!("解析 threads 列失败: {}", e))?,
            );
        }

        if columns.is_empty() {
            return Err("threads 表不存在或没有列定义".to_string());
        }

        Ok(columns)
    }

    fn quote_identifier(value: &str) -> String {
        format!("\"{}\"", value.replace('"', "\"\""))
    }

    fn read_codex_session_index_map(path: &PathBuf) -> HashMap<String, JsonValue> {
        let mut entries = HashMap::new();
        let Ok(content) = fs::read_to_string(path) else {
            return entries;
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) else {
                continue;
            };
            let Some(id) = value.get("id").and_then(|item| item.as_str()) else {
                continue;
            };
            entries.insert(id.to_string(), value);
        }

        entries
    }

    pub(super) fn remove_codex_threads_for_instance(
        instance: &CodexInstanceRef,
        session_ids: &[String],
    ) -> Result<(), String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        if !db_path.exists() {
            return Ok(());
        }

        let mut connection = Connection::open(&db_path)
            .map_err(|e| format!("打开 Codex 数据库失败 ({}): {}", db_path.display(), e))?;
        let tx = connection
            .transaction()
            .map_err(|e| format!("开启 Codex 会话删除事务失败: {}", e))?;

        for session_id in session_ids {
            tx.execute("DELETE FROM threads WHERE id = ?1", [session_id.as_str()])
                .map_err(|e| format!("删除 Codex 会话记录失败 ({}): {}", session_id, e))?;
        }

        tx.commit()
            .map_err(|e| format!("提交 Codex 会话删除事务失败: {}", e))?;
        Ok(())
    }

    pub(super) fn rewrite_codex_session_index_without_ids_for_instance(
        instance: &CodexInstanceRef,
        session_ids: &[String],
    ) -> Result<(), String> {
        let path = instance.data_dir.join("session_index.jsonl");
        if !path.exists() {
            return Ok(());
        }

        let removed_ids = session_ids.iter().map(|id| id.as_str()).collect::<HashSet<_>>();
        let content = fs::read_to_string(&path).map_err(|e| {
            format!(
                "读取 Codex session_index.jsonl 失败 ({}): {}",
                path.display(),
                e
            )
        })?;

        let retained = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return false;
                }
                match serde_json::from_str::<JsonValue>(trimmed) {
                    Ok(value) => value
                        .get("id")
                        .and_then(JsonValue::as_str)
                        .map(|id| !removed_ids.contains(id))
                        .unwrap_or(true),
                    Err(_) => true,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let final_content = if retained.is_empty() {
            String::new()
        } else {
            format!("{}\n", retained)
        };

        fs::write(&path, final_content).map_err(|e| {
            format!(
                "重写 Codex session_index.jsonl 失败 ({}): {}",
                path.display(),
                e
            )
        })?;
        Ok(())
    }

    fn format_index_timestamp(updated_at: i64) -> String {
        chrono::DateTime::<Utc>::from_timestamp(updated_at, 0)
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    fn repair_codex_session_index_for_instance(
        instance: &CodexInstanceRef,
    ) -> Result<(usize, usize, usize), String> {
        let db_path = instance.data_dir.join("state_5.sqlite");
        let index_path = instance.data_dir.join("session_index.jsonl");

        if !db_path.exists() {
            return Ok((0, 0, 0));
        }

        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("打开 Codex 数据库失败 ({}): {}", db_path.display(), e))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, title, first_user_message, updated_at
             FROM threads
             WHERE archived = 0
             ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("读取 Codex 线程失败 ({}): {}", instance.name, e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| format!("查询 Codex 线程失败 ({}): {}", instance.name, e))?;

        let mut thread_map = HashMap::<String, JsonValue>::new();
        for row in rows.flatten() {
            let (id, title, first_user_message, updated_at) = row;
            let final_title = Self::sanitize_session_title(title)
                .or_else(|| Self::sanitize_session_title(first_user_message))
                .unwrap_or_else(|| id.clone());
            thread_map.insert(
                id.clone(),
                serde_json::json!({
                    "id": id,
                    "thread_name": final_title,
                    "updated_at": Self::format_index_timestamp(updated_at),
                }),
            );
        }

        let existing_content = fs::read_to_string(&index_path).unwrap_or_default();
        let mut existing_map = HashMap::<String, JsonValue>::new();
        for line in existing_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) else {
                continue;
            };
            let Some(id) = value.get("id").and_then(|item| item.as_str()) else {
                continue;
            };
            existing_map.insert(id.to_string(), value);
        }

        let thread_count = thread_map.len();
        let added_index_entries = thread_map
            .keys()
            .filter(|id| !existing_map.contains_key(*id))
            .count();
        let removed_stale_entries = existing_map
            .keys()
            .filter(|id| !thread_map.contains_key(*id))
            .count();

        let mut merged = thread_map.into_iter().collect::<Vec<_>>();
        merged.sort_by(|a, b| a.0.cmp(&b.0));
        let final_content = merged
            .into_iter()
            .map(|(_, value)| serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建 Codex 索引目录失败 ({}): {}", parent.display(), e))?;
        }
        fs::write(
            &index_path,
            if final_content.is_empty() {
                String::new()
            } else {
                format!("{}\n", final_content)
            },
        )
        .map_err(|e| {
            format!(
                "写入 Codex session_index.jsonl 失败 ({}): {}",
                index_path.display(),
                e
            )
        })?;

        Ok((thread_count, added_index_entries, removed_stale_entries))
    }

    fn sync_missing_codex_threads_to_instance(
        target: &CodexInstanceRef,
        snapshots: &[ThreadSnapshot],
    ) -> Result<PathBuf, String> {
        let backup_dir = Self::backup_codex_instance_files(&target.data_dir)?;
        let existing_index_ids =
            Self::read_codex_session_index_map(&target.data_dir.join("session_index.jsonl"))
                .keys()
                .cloned()
                .collect::<HashSet<_>>();
        let db_path = target.data_dir.join("state_5.sqlite");
        let mut connection = Connection::open(&db_path)
            .map_err(|e| format!("打开目标实例数据库失败 ({}): {}", target.name, e))?;
        let target_columns = Self::read_thread_columns(&connection)?;
        let transaction = connection
            .transaction()
            .map_err(|e| format!("开启目标实例事务失败 ({}): {}", target.name, e))?;

        for snapshot in snapshots {
            let target_rollout_path = Self::copy_codex_rollout_file(snapshot, &target.data_dir)?;
            let mut row_data = snapshot.row_data.clone();
            row_data.set_text(
                "rollout_path",
                target_rollout_path.to_string_lossy().to_string(),
            );
            Self::insert_codex_thread_row(&transaction, &target_columns, &row_data)?;
        }

        transaction
            .commit()
            .map_err(|e| format!("提交目标实例事务失败 ({}): {}", target.name, e))?;

        Self::append_codex_session_index_entries(
            &target.data_dir.join("session_index.jsonl"),
            &existing_index_ids,
            snapshots,
        )?;
        Ok(backup_dir)
    }

    fn backup_codex_instance_files(data_dir: &PathBuf) -> Result<PathBuf, String> {
        let backup_dir = data_dir.join(format!(
            "backup-{}-instance-thread-sync",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("创建备份目录失败 ({}): {}", data_dir.display(), e))?;

        for file_name in ["state_5.sqlite", "session_index.jsonl"] {
            let source = data_dir.join(file_name);
            if !source.exists() {
                continue;
            }
            let target = backup_dir.join(format!("{}.bak", file_name));
            fs::copy(&source, &target).map_err(|e| {
                format!(
                    "备份文件失败 ({} -> {}): {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;
        }

        Ok(backup_dir)
    }

    fn copy_codex_rollout_file(
        snapshot: &ThreadSnapshot,
        target_root: &PathBuf,
    ) -> Result<PathBuf, String> {
        let relative = snapshot
            .rollout_path
            .strip_prefix(&snapshot.source_root)
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|_| {
                PathBuf::from("sessions").join(
                    snapshot
                        .rollout_path
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("rollout.jsonl")),
                )
            });
        let target_path = target_root.join(relative);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目标线程目录失败 ({}): {}", parent.display(), e))?;
        }
        if !target_path.exists() {
            fs::copy(&snapshot.rollout_path, &target_path).map_err(|e| {
                format!(
                    "复制线程文件失败 ({} -> {}): {}",
                    snapshot.rollout_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        }
        Ok(target_path)
    }

    fn insert_codex_thread_row(
        transaction: &Transaction<'_>,
        target_columns: &[String],
        row_data: &ThreadRowData,
    ) -> Result<(), String> {
        let mut values = Vec::with_capacity(target_columns.len());
        for column in target_columns {
            values.push(row_data.get_value(column).cloned().unwrap_or(Value::Null));
        }

        let quoted_columns = target_columns
            .iter()
            .map(|column| Self::quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = std::iter::repeat("?")
            .take(target_columns.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT OR IGNORE INTO threads ({}) VALUES ({})",
            quoted_columns, placeholders
        );

        transaction
            .execute(&query, params_from_iter(values.iter()))
            .map_err(|e| format!("写入目标线程记录失败: {}", e))?;
        Ok(())
    }

    fn append_codex_session_index_entries(
        index_path: &PathBuf,
        existing_ids: &HashSet<String>,
        snapshots: &[ThreadSnapshot],
    ) -> Result<(), String> {
        let mut lines = Vec::new();
        for snapshot in snapshots {
            if existing_ids.contains(&snapshot.id) {
                continue;
            }
            lines.push(
                serde_json::to_string(&snapshot.session_index_entry)
                    .map_err(|e| format!("序列化 session_index 条目失败: {}", e))?,
            );
        }
        if lines.is_empty() {
            return Ok(());
        }

        let needs_prefix = index_path.exists() && !Self::file_ends_with_newline(index_path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(index_path)
            .map_err(|e| {
                format!(
                    "打开 session_index.jsonl 失败 ({}): {}",
                    index_path.display(),
                    e
                )
            })?;

        use std::io::Write;
        if needs_prefix {
            file.write_all(b"\n").map_err(|e| {
                format!(
                    "写入 session_index 换行失败 ({}): {}",
                    index_path.display(),
                    e
                )
            })?;
        }
        for line in lines {
            file.write_all(line.as_bytes())
                .and_then(|_| file.write_all(b"\n"))
                .map_err(|e| {
                    format!(
                        "追加 session_index 条目失败 ({}): {}",
                        index_path.display(),
                        e
                    )
                })?;
        }
        Ok(())
    }

    fn file_ends_with_newline(path: &PathBuf) -> Result<bool, String> {
        let bytes =
            fs::read(path).map_err(|e| format!("读取文件失败 ({}): {}", path.display(), e))?;
        Ok(bytes.last().copied() == Some(b'\n'))
    }
}
