use crate::{
    db::Database,
    error::AppResult,
    models::{IpAccessLog, IpRule},
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct SecurityDbService {
    db: Arc<Database>,
}

impl SecurityDbService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { db: db.clone() }
    }

    /// 记录一条 IP 访问日志（供前端网关实时调度）
    pub fn log_access(
        &self,
        ip_address: &str,
        endpoint: &str,
        token_id: Option<&str>,
        action_taken: &str,
        reason: Option<&str>,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        self.db.execute(
            "INSERT INTO ip_access_logs (id, ip_address, endpoint, token_id, action_taken, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                &id,
                &ip_address,
                &endpoint,
                &token_id.unwrap_or(""),
                &action_taken,
                &reason.unwrap_or(""),
                &now,
            ],
        )?;

        // 防爆库：随机概率（或每次）删掉最旧的记录保底，比如只保留 10000 条
        // 为了避免每次插入都查数量导致慢，可以使用时间来切断
        let two_days_ago = now - 2 * 24 * 3600;
        let _ = self.db.execute(
            "DELETE FROM ip_access_logs WHERE created_at < ?1",
            &[&two_days_ago],
        );

        Ok(())
    }

    pub fn get_access_logs(&self, limit: i64) -> AppResult<Vec<IpAccessLog>> {
        let logs = self.db.query_rows(
            "SELECT id, ip_address, endpoint, token_id, action_taken, reason, created_at 
             FROM ip_access_logs ORDER BY created_at DESC LIMIT ?1",
            &[&limit],
            |row| {
                Ok(IpAccessLog {
                    id: row.get(0)?,
                    ip_address: row.get(1)?,
                    endpoint: row.get(2)?,
                    token_id: {
                        let t: String = row.get(3)?;
                        if t.is_empty() {
                            None
                        } else {
                            Some(t)
                        }
                    },
                    action_taken: row.get(4)?,
                    reason: {
                        let r: String = row.get(5)?;
                        if r.is_empty() {
                            None
                        } else {
                            Some(r)
                        }
                    },
                    created_at: row.get(6)?,
                })
            },
        )?;
        Ok(logs)
    }

    pub fn clear_access_logs(&self) -> AppResult<()> {
        self.db.execute("DELETE FROM ip_access_logs", &[])?;
        Ok(())
    }

    // ============================================
    // IP 规则（黑名单/白名单）
    // ============================================

    pub fn get_all_rules(&self) -> AppResult<Vec<IpRule>> {
        let rules = self.db.query_rows(
            "SELECT id, ip_cidr, rule_type, notes, is_active, created_at FROM ip_rules ORDER BY created_at DESC",
            &[],
            |r| {
                Ok(IpRule {
                    id: r.get(0)?,
                    ip_cidr: r.get(1)?,
                    rule_type: r.get(2)?,
                    notes: {
                        let n: String = r.get(3)?;
                        if n.is_empty() { None } else { Some(n) }
                    },
                    is_active: r.get(4)?,
                    created_at: r.get(5)?,
                })
            },
        )?;
        Ok(rules)
    }

    pub fn add_rule(&self, ip_cidr: &str, rule_type: &str, notes: Option<&str>) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        self.db.execute(
            "INSERT INTO ip_rules (id, ip_cidr, rule_type, notes, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            &[&id, &ip_cidr, &rule_type, &notes.unwrap_or(""), &now],
        )?;
        // Update security cache right away in proxy
        crate::proxy::security::SecurityShield::sync_rules(&self.db)?;
        Ok(())
    }

    pub fn delete_rule(&self, id: &str) -> AppResult<()> {
        self.db
            .execute("DELETE FROM ip_rules WHERE id = ?1", &[&id])?;
        crate::proxy::security::SecurityShield::sync_rules(&self.db)?;
        Ok(())
    }

    pub fn toggle_rule(&self, id: &str, active: bool) -> AppResult<()> {
        let act = if active { 1 } else { 0 };
        self.db.execute(
            "UPDATE ip_rules SET is_active = ?1 WHERE id = ?2",
            &[&act, &id],
        )?;
        crate::proxy::security::SecurityShield::sync_rules(&self.db)?;
        Ok(())
    }
}
