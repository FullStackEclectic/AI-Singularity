use crate::db::Database;
use crate::models::{CreateUserTokenReq, UpdateUserTokenReq, UserToken};
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserTokenService<'a> {
    db: &'a Database,
}

impl<'a> UserTokenService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    fn generate_sk_token() -> String {
        format!("sk-ag-{}", Uuid::new_v4().to_string().replace("-", ""))
    }

    pub fn create_token(&self, req: CreateUserTokenReq) -> Result<UserToken> {
        let mut id = Uuid::new_v4().to_string();
        let mut token_str = Self::generate_sk_token();
        let now = Utc::now().timestamp();

        let user_token = UserToken {
            id: id.clone(),
            token: token_str,
            username: req.username,
            description: req.description,
            enabled: true,
            expires_type: req.expires_type,
            expires_at: req.expires_at,
            max_ips: req.max_ips,
            curfew_start: req.curfew_start,
            curfew_end: req.curfew_end,
            total_requests: 0,
            total_tokens_used: 0,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        };

        self.db.execute(
            "INSERT INTO user_tokens (id, token, username, description, enabled, expires_type, expires_at, max_ips, curfew_start, curfew_end, total_requests, total_tokens_used, created_at, updated_at, last_used_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                user_token.id,
                user_token.token,
                user_token.username,
                user_token.description,
                user_token.enabled,
                user_token.expires_type,
                user_token.expires_at,
                user_token.max_ips,
                user_token.curfew_start,
                user_token.curfew_end,
                user_token.total_requests,
                user_token.total_tokens_used,
                user_token.created_at,
                user_token.updated_at,
                user_token.last_used_at,
            ],
        )?;

        Ok(user_token)
    }

    pub fn get_all_tokens(&self) -> Result<Vec<UserToken>> {
        self.db.query_rows(
            "SELECT id, token, username, description, enabled, expires_type, expires_at, max_ips, curfew_start, curfew_end, total_requests, total_tokens_used, created_at, updated_at, last_used_at FROM user_tokens ORDER BY created_at DESC",
            &[],
            |row| {
                Ok(UserToken {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    username: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get(4)?,
                    expires_type: row.get(5)?,
                    expires_at: row.get(6)?,
                    max_ips: row.get(7)?,
                    curfew_start: row.get(8)?,
                    curfew_end: row.get(9)?,
                    total_requests: row.get(10)?,
                    total_tokens_used: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                    last_used_at: row.get(14)?,
                })
            },
        ).context("Failed to get all user tokens")
    }

    pub fn get_token_by_str(&self, token_str: &str) -> Result<Option<UserToken>> {
        let rows = self.db.query_rows(
            "SELECT id, token, username, description, enabled, expires_type, expires_at, max_ips, curfew_start, curfew_end, total_requests, total_tokens_used, created_at, updated_at, last_used_at FROM user_tokens WHERE token = ?1",
            &[&token_str],
            |row| {
                Ok(UserToken {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    username: row.get(2)?,
                    description: row.get(3)?,
                    enabled: row.get(4)?,
                    expires_type: row.get(5)?,
                    expires_at: row.get(6)?,
                    max_ips: row.get(7)?,
                    curfew_start: row.get(8)?,
                    curfew_end: row.get(9)?,
                    total_requests: row.get(10)?,
                    total_tokens_used: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                    last_used_at: row.get(14)?,
                })
            },
        )?;
        Ok(rows.into_iter().next())
    }

    pub fn update_token(&self, req: UpdateUserTokenReq) -> Result<()> {
        let mut updates = Vec::new();
        let mut p_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut p_idx = 1;

        if let Some(username) = req.username {
            updates.push(format!("username = ?{}", p_idx));
            p_values.push(Box::new(username));
            p_idx += 1;
        }
        if let Some(desc) = req.description {
            updates.push(format!("description = ?{}", p_idx));
            p_values.push(Box::new(desc));
            p_idx += 1;
        }
        if let Some(enabled) = req.enabled {
            updates.push(format!("enabled = ?{}", p_idx));
            p_values.push(Box::new(enabled));
            p_idx += 1;
        }
        if let Some(et) = req.expires_type {
            updates.push(format!("expires_type = ?{}", p_idx));
            p_values.push(Box::new(et));
            p_idx += 1;
        }
        if let Some(ea) = req.expires_at {
            updates.push(format!("expires_at = ?{}", p_idx));
            p_values.push(Box::new(ea));
            p_idx += 1;
        }
        if let Some(mi) = req.max_ips {
            updates.push(format!("max_ips = ?{}", p_idx));
            p_values.push(Box::new(mi));
            p_idx += 1;
        }
        if let Some(cs) = req.curfew_start {
            updates.push(format!("curfew_start = ?{}", p_idx));
            p_values.push(Box::new(cs));
            p_idx += 1;
        }
        if let Some(ce) = req.curfew_end {
            updates.push(format!("curfew_end = ?{}", p_idx));
            p_values.push(Box::new(ce));
            p_idx += 1;
        }

        if updates.is_empty() {
            return Ok(());
        }

        let now = Utc::now().timestamp();
        updates.push(format!("updated_at = ?{}", p_idx));
        p_values.push(Box::new(now));
        p_idx += 1;

        let query = format!(
            "UPDATE user_tokens SET {} WHERE id = ?{}",
            updates.join(", "),
            p_idx
        );
        p_values.push(Box::new(req.id));

        // convert to vec of references
        let params_refs: Vec<&dyn rusqlite::ToSql> = p_values.iter().map(|b| b.as_ref()).collect();

        self.db.execute(&query, &params_refs)?;
        Ok(())
    }

    pub fn delete_token(&self, id: &str) -> Result<()> {
        self.db.execute("DELETE FROM user_tokens WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn increment_token_usage(&self, id: &str, req_count: i64, tokens_count: i64) -> Result<()> {
        let now = Utc::now().timestamp();
        self.db.execute(
            "UPDATE user_tokens SET total_requests = total_requests + ?1, total_tokens_used = total_tokens_used + ?2, last_used_at = ?3 WHERE id = ?4",
            params![req_count, tokens_count, now, id],
        )?;
        Ok(())
    }

    pub fn get_summary(&self) -> Result<crate::models::UserTokenSummary> {
        let total_tokens: i64 = self.db.query_scalar("SELECT COUNT(*) FROM user_tokens", &[])?;
        let active_tokens: i64 = self.db.query_scalar("SELECT COUNT(*) FROM user_tokens WHERE enabled = 1", &[])?;
        let total_users: i64 = self.db.query_scalar("SELECT COUNT(DISTINCT username) FROM user_tokens", &[])?;
        
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let today_requests: i64 = self.db.query_scalar(
            "SELECT COALESCE(SUM(total_requests), 0) FROM user_tokens WHERE last_used_at >= ?", 
            &[&today_start]
        ).unwrap_or(0);

        Ok(crate::models::UserTokenSummary {
            total_tokens,
            active_tokens,
            total_users,
            today_requests,
        })
    }
}
