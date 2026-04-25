use super::Database;
use chrono::Utc;
use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedCodexQuota {
    pub account_id: String,
    pub plan_type: Option<String>,
    pub body_json: String,
    pub fetched_at: String,
    pub expires_at: String,
    pub hit_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaCacheStats {
    pub total: i64,
    pub valid: i64,
    pub hit_total: i64,
    pub last_written_at: Option<String>,
}

fn map_row(row: &rusqlite::Row<'_>) -> SqlResult<CachedCodexQuota> {
    Ok(CachedCodexQuota {
        account_id: row.get(0)?,
        plan_type: row.get(1)?,
        body_json: row.get(2)?,
        fetched_at: row.get(3)?,
        expires_at: row.get(4)?,
        hit_count: row.get(5)?,
    })
}

impl Database {
    /// Read a non-expired cache row. Returns None if missing or already expired.
    pub fn read_codex_quota_cache(
        &self,
        account_id: &str,
    ) -> SqlResult<Option<CachedCodexQuota>> {
        let now = Utc::now().to_rfc3339();
        Ok(self
            .query_row(
                "SELECT account_id, plan_type, body_json, fetched_at, expires_at, hit_count
                 FROM codex_quota_cache
                 WHERE account_id = ?1 AND expires_at > ?2",
                &[&account_id, &now.as_str()],
                map_row,
            )
            .ok())
    }

    pub fn write_codex_quota_cache(
        &self,
        account_id: &str,
        plan_type: Option<&str>,
        body_json: &str,
        ttl_seconds: i64,
    ) -> SqlResult<usize> {
        let now = Utc::now();
        let fetched_at = now.to_rfc3339();
        let expires_at = (now + chrono::Duration::seconds(ttl_seconds.max(1))).to_rfc3339();
        self.execute(
            "INSERT INTO codex_quota_cache (account_id, plan_type, body_json, fetched_at, expires_at, hit_count)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)
             ON CONFLICT(account_id) DO UPDATE SET
                plan_type = excluded.plan_type,
                body_json = excluded.body_json,
                fetched_at = excluded.fetched_at,
                expires_at = excluded.expires_at,
                hit_count = 0",
            rusqlite::params![account_id, plan_type, body_json, fetched_at, expires_at],
        )
    }

    pub fn bump_codex_quota_cache_hit(&self, account_id: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE codex_quota_cache SET hit_count = hit_count + 1 WHERE account_id = ?1",
            &[&account_id],
        )
    }

    pub fn delete_codex_quota_cache(&self, account_id: Option<&str>) -> SqlResult<usize> {
        match account_id {
            Some(id) => self.execute(
                "DELETE FROM codex_quota_cache WHERE account_id = ?1",
                &[&id],
            ),
            None => self.execute("DELETE FROM codex_quota_cache", &[]),
        }
    }

    pub fn codex_quota_cache_stats(&self) -> SqlResult<CodexQuotaCacheStats> {
        let now = Utc::now().to_rfc3339();
        let total: i64 = self
            .query_scalar("SELECT COUNT(*) FROM codex_quota_cache", &[])
            .unwrap_or(0);
        let valid: i64 = self
            .query_scalar(
                "SELECT COUNT(*) FROM codex_quota_cache WHERE expires_at > ?1",
                &[&now.as_str()],
            )
            .unwrap_or(0);
        let hit_total: i64 = self
            .query_scalar("SELECT COALESCE(SUM(hit_count), 0) FROM codex_quota_cache", &[])
            .unwrap_or(0);
        let last_written_at: Option<String> = self
            .query_row(
                "SELECT MAX(fetched_at) FROM codex_quota_cache",
                &[],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        Ok(CodexQuotaCacheStats {
            total,
            valid,
            hit_total,
            last_written_at,
        })
    }
}
