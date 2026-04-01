use anyhow::Result;
use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        // schema_version 表：追踪迁移版本
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version     INTEGER PRIMARY KEY,
                applied_at  TEXT NOT NULL
            );",
        )?;

        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // ── Migration 1: 基础表结构 ──────────────────────────────────────
        if current_version < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS api_keys (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    platform        TEXT NOT NULL,
                    base_url        TEXT,
                    key_hash        TEXT NOT NULL,
                    key_preview     TEXT NOT NULL,
                    status          TEXT NOT NULL DEFAULT 'unknown',
                    notes           TEXT,
                    created_at      TEXT NOT NULL,
                    last_checked_at TEXT
                );

                CREATE TABLE IF NOT EXISTS providers (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    platform        TEXT NOT NULL DEFAULT 'custom',
                    category        TEXT,
                    base_url        TEXT,
                    api_key_id      TEXT,
                    model_name      TEXT NOT NULL DEFAULT '',
                    is_active       INTEGER NOT NULL DEFAULT 0,
                    tool_targets    TEXT,
                    icon            TEXT,
                    icon_color      TEXT,
                    website_url     TEXT,
                    api_key_url     TEXT,
                    notes           TEXT,
                    extra_config    TEXT,
                    created_at      TEXT NOT NULL,
                    updated_at      TEXT NOT NULL,
                    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE SET NULL
                );

                CREATE TABLE IF NOT EXISTS mcp_servers (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    command         TEXT NOT NULL,
                    args            TEXT,
                    env             TEXT,
                    description     TEXT,
                    is_active       INTEGER NOT NULL DEFAULT 1,
                    tool_targets    TEXT,
                    created_at      TEXT NOT NULL,
                    updated_at      TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS prompts (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    target_file     TEXT NOT NULL,
                    content         TEXT NOT NULL,
                    is_active       INTEGER NOT NULL DEFAULT 1,
                    created_at      TEXT NOT NULL,
                    updated_at      TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS balance_snapshots (
                    id              TEXT PRIMARY KEY,
                    provider_id     TEXT NOT NULL,
                    provider_name   TEXT NOT NULL,
                    balance_usd     REAL,
                    balance_cny     REAL,
                    quota_remaining REAL,
                    quota_unit      TEXT,
                    quota_reset_at  TEXT,
                    snapped_at      TEXT NOT NULL
                );

                INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'));",
            )?;
        }

        // ── Migration 2: providers 旧字段兼容（若已有旧表则迁移字段）───────
        if current_version < 2 {
            // ALTER TABLE 忽略"已存在"错误（rusqlite 会 Err，用 ignore）
            let _ = conn.execute_batch(
                "ALTER TABLE providers ADD COLUMN category TEXT;
                 ALTER TABLE providers ADD COLUMN tool_targets TEXT;
                 ALTER TABLE providers ADD COLUMN icon TEXT;
                 ALTER TABLE providers ADD COLUMN icon_color TEXT;
                 ALTER TABLE providers ADD COLUMN website_url TEXT;
                 ALTER TABLE providers ADD COLUMN api_key_url TEXT;
                 ALTER TABLE providers ADD COLUMN extra_config TEXT;",
            );
            let _ = conn.execute_batch(
                "ALTER TABLE mcp_servers ADD COLUMN description TEXT;
                 ALTER TABLE mcp_servers ADD COLUMN tool_targets TEXT;",
            );
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS balance_snapshots (
                    id              TEXT PRIMARY KEY,
                    provider_id     TEXT NOT NULL,
                    provider_name   TEXT NOT NULL,
                    balance_usd     REAL,
                    balance_cny     REAL,
                    quota_remaining REAL,
                    quota_unit      TEXT,
                    quota_reset_at  TEXT,
                    snapped_at      TEXT NOT NULL
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (2, datetime('now'));",
            )?;
        }

        Ok(())
    }

    /// 执行不返回数据的 SQL 语句
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(sql, params)
    }

    /// 执行查询并返回多行数据（通过回调提取每行）
    pub fn query_rows<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> SqlResult<Vec<T>>
    where
        F: Fn(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params, f)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 执行查询并返回单行数据
    pub fn query_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> SqlResult<T>
    where
        F: Fn(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, f)
    }

    /// 执行查询返回单列单行（常用于 COUNT）
    pub fn query_scalar<T: rusqlite::types::FromSql>(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> SqlResult<T> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, |r| r.get(0))
    }
}
