use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        // API Keys 表
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                platform    TEXT NOT NULL,
                base_url    TEXT,
                key_hash    TEXT NOT NULL,   -- SHA-256 哈希，用于去重
                key_preview TEXT NOT NULL,   -- 前8位 + '...'
                status      TEXT NOT NULL DEFAULT 'unknown',
                notes       TEXT,
                created_at  TEXT NOT NULL,
                last_checked_at TEXT
            );",
        )?;

        // 余额缓存表
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

        // 模型缓存表
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

        // 使用记录表（用于本地统计）
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

    pub fn conn(&self) -> std::sync::MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }
}
