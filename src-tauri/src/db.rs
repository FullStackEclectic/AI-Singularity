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
            );",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS balances (
                key_id          TEXT PRIMARY KEY,
                platform        TEXT NOT NULL,
                balance_usd     REAL,
                balance_cny     REAL,
                total_usage_usd REAL,
                quota_remaining REAL,
                quota_reset_at  TEXT,
                synced_at       TEXT NOT NULL,
                FOREIGN KEY (key_id) REFERENCES api_keys(id) ON DELETE CASCADE
            );",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS models (
                id                    TEXT NOT NULL,
                platform              TEXT NOT NULL,
                name                  TEXT NOT NULL,
                context_length        INTEGER,
                supports_vision       INTEGER NOT NULL DEFAULT 0,
                supports_tools        INTEGER NOT NULL DEFAULT 0,
                input_price_per_1m    REAL,
                output_price_per_1m   REAL,
                is_available          INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (id, platform)
            );",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_logs (
                id              TEXT PRIMARY KEY,
                key_id          TEXT NOT NULL,
                model_id        TEXT NOT NULL,
                platform        TEXT NOT NULL,
                prompt_tokens   INTEGER NOT NULL DEFAULT 0,
                output_tokens   INTEGER NOT NULL DEFAULT 0,
                cost_usd        REAL,
                recorded_at     TEXT NOT NULL,
                FOREIGN KEY (key_id) REFERENCES api_keys(id) ON DELETE CASCADE
            );",
        )?;

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
