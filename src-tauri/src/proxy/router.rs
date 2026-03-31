/// 配额感知路由引擎
use crate::db::Database;
use crate::models::Platform;
use crate::store::SecureStore;
use std::sync::Arc;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct RouteTarget {
    pub key_id: String,
    pub secret: String,
    pub platform: Platform,
    pub base_url: Option<String>,
}

pub struct Router {
    db: Arc<Database>,
}

impl Router {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 选择最优 Key（valid 状态，最近检测的排前面）
    pub fn pick_best_key(&self, platform: Option<&str>) -> Option<RouteTarget> {
        let sql = if let Some(p) = platform {
            format!(
                "SELECT id, platform, base_url FROM api_keys
                 WHERE status = 'valid' AND platform = '{}'
                 ORDER BY last_checked_at DESC LIMIT 1",
                p
            )
        } else {
            "SELECT id, platform, base_url FROM api_keys
             WHERE status = 'valid'
             ORDER BY last_checked_at DESC LIMIT 1".to_string()
        };

        let rows: Vec<(String, String, Option<String>)> = self
            .db
            .query_rows(&sql, &[], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap_or_default();

        rows.into_iter().next().and_then(|(id, platform_str, base_url)| {
            let secret = SecureStore::get_key(&id).ok()?;
            let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
                .unwrap_or(Platform::Custom);
            Some(RouteTarget { key_id: id, secret, platform, base_url })
        })
    }

    /// 标记 Key 状态（请求失败时调用）
    pub fn mark_key_status(&self, key_id: &str, status: &str) {
        let _ = self.db.execute(
            "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
            &[&status, &Utc::now().to_rfc3339(), &key_id],
        );
    }
}
