use rusqlite::{Connection, Result as SqlResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

mod ide_accounts;
mod migrations;

pub struct Database {
    pub path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            conn: self.conn.clone(),
        }
    }
}

impl Database {
    pub fn new(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            path: path.to_path_buf(),
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(sql, params)
    }

    pub fn query_row<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> SqlResult<T>
    where
        F: FnOnce(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, f)
    }

    pub fn query_rows<T, F>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
        f: F,
    ) -> SqlResult<Vec<T>>
    where
        F: Fn(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, f)?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    pub fn query_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> SqlResult<T>
    where
        F: Fn(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, f)
    }

    pub fn query_scalar<T: rusqlite::types::FromSql>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> SqlResult<T> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, |r| r.get(0))
    }

    pub fn recover_rate_limited_nodes(&self) {
        let conn = self.conn.lock().unwrap();
        let api_key_sql = "UPDATE api_keys
                           SET status = 'valid'
                           WHERE status = 'rate_limit'
                           AND last_checked_at < datetime('now', '-5 minutes');";

        let api_rows = conn.execute(api_key_sql, []).unwrap_or(0);
        if api_rows > 0 {
            tracing::info!(
                "♻️ [流量重塑] 成功复活了 {} 个进入冷却期的 API Key 节点",
                api_rows
            );
        }

        let ide_account_sql = "UPDATE ide_accounts
                               SET status = 'active'
                               WHERE status = 'rate_limited'
                               AND updated_at < datetime('now', '-5 minutes');";

        let ide_rows = conn.execute(ide_account_sql, []).unwrap_or(0);
        if ide_rows > 0 {
            tracing::info!(
                "♻️ [降维预警解除] 成功复活了 {} 个高优 IDE 伪装节点的 Rate Limit",
                ide_rows
            );
        }
    }

    pub fn update_api_key_tags(&self, id: &str, tags_json: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE api_keys SET tags = ? WHERE id = ?",
            &[&tags_json, &id],
        )
    }
}
