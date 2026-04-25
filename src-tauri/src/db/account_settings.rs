use super::Database;
use chrono::Utc;
use rusqlite::Result as SqlResult;
use std::collections::HashMap;

impl Database {
    pub fn get_account_setting(&self, key: &str) -> SqlResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT value FROM account_settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }

    pub fn get_all_account_settings(&self) -> SqlResult<HashMap<String, String>> {
        self.query_rows(
            "SELECT key, value FROM account_settings",
            &[],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map(|rows| rows.into_iter().collect())
    }

    pub fn set_account_setting(&self, key: &str, value: &str) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.execute(
            "INSERT INTO account_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            &[&key, &value, &now],
        )
    }

    pub fn set_account_settings_batch(&self, kvs: &[(String, String)]) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for (k, v) in kvs {
            tx.execute(
                "INSERT INTO account_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![k, v, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
