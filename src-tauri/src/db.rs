use anyhow::Result;
use rusqlite::{Connection, Result as SqlResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

                CREATE TABLE IF NOT EXISTS alert_history (
                    alert_id        TEXT PRIMARY KEY,
                    last_sent_at    TEXT NOT NULL
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
                "ALTER TABLE prompts ADD COLUMN description TEXT;",
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
        
        // ── Migration 3: Token 用量审计表 ─────────────────────────────────
        if current_version < 3 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS token_usage_records (
                    id                  TEXT PRIMARY KEY,
                    key_id              TEXT NOT NULL,
                    platform            TEXT NOT NULL,
                    model_name          TEXT NOT NULL,
                    client_app          TEXT NOT NULL,
                    prompt_tokens       INTEGER NOT NULL DEFAULT 0,
                    completion_tokens   INTEGER NOT NULL DEFAULT 0,
                    total_tokens        INTEGER NOT NULL DEFAULT 0,
                    created_at          TEXT NOT NULL
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (3, datetime('now'));",
            )?;
        }
        
        // ── Migration 4: 降维打击武器库体系表 (IDE 指纹账号轮询池) ─────────────────────────────────
        if current_version < 4 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS ide_accounts (
                    id                  TEXT PRIMARY KEY,
                    email               TEXT NOT NULL,
                    origin_platform     TEXT NOT NULL,
                    access_token        TEXT NOT NULL,
                    refresh_token       TEXT NOT NULL,
                    expires_in          INTEGER NOT NULL DEFAULT 0,
                    token_type          TEXT NOT NULL,
                    status              TEXT NOT NULL DEFAULT 'active',
                    disabled_reason     TEXT,
                    is_proxy_disabled   BOOLEAN NOT NULL DEFAULT 0,
                    machine_id          TEXT,
                    mac_machine_id      TEXT,
                    dev_device_id       TEXT,
                    sqm_id              TEXT,
                    quota_json          TEXT,
                    created_at          TEXT NOT NULL,
                    updated_at          TEXT NOT NULL,
                    last_used           TEXT NOT NULL
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (4, datetime('now'));",
            )?;
        }

        // ── Migration 5: SaaS User Tokens (用于向下级分发的子账号凭证) ─────────────────────────────────
        if current_version < 5 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS user_tokens (
                    id                  TEXT PRIMARY KEY,
                    token               TEXT NOT NULL UNIQUE,
                    username            TEXT NOT NULL,
                    description         TEXT,
                    enabled             BOOLEAN NOT NULL DEFAULT 1,
                    expires_type        TEXT NOT NULL DEFAULT 'never',
                    expires_at          INTEGER,
                    max_ips             INTEGER NOT NULL DEFAULT 0,
                    curfew_start        TEXT,
                    curfew_end          TEXT,
                    total_requests      INTEGER NOT NULL DEFAULT 0,
                    total_tokens_used   INTEGER NOT NULL DEFAULT 0,
                    created_at          INTEGER NOT NULL,
                    updated_at          INTEGER NOT NULL,
                    last_used_at        INTEGER
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (5, datetime('now'));",
            )?;
        }

        // ── Migration 6: api_keys 优先级字段 + prompts tool_targets 补充 ─────
        if current_version < 6 {
            // 忽略"column already exists"错误
            let _ = conn.execute_batch(
                "ALTER TABLE api_keys ADD COLUMN priority INTEGER NOT NULL DEFAULT 100;",
            );
            let _ = conn.execute_batch(
                "ALTER TABLE prompts ADD COLUMN tool_targets TEXT;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (6, datetime('now'));",
            )?;
        }

        // ── Migration 7: 账号标签（tags）字段 ─────────────────────────────────
        if current_version < 7 {
            let _ = conn.execute_batch(
                "ALTER TABLE ide_accounts ADD COLUMN tags TEXT;
                 ALTER TABLE api_keys ADD COLUMN tags TEXT;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (7, datetime('now'));",
            )?;
        }

        // ── Migration 8: 高级网关特性 - 自定义模型映射表 (Custom Model Mappings) ─────────────
        if current_version < 8 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS model_mappings (
                    id              TEXT PRIMARY KEY,
                    source_model    TEXT NOT NULL,
                    target_model    TEXT NOT NULL,
                    is_active       BOOLEAN NOT NULL DEFAULT 1,
                    created_at      TEXT NOT NULL,
                    updated_at      TEXT NOT NULL
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (8, datetime('now'));",
            )?;
        }

        // ── Migration 9: Providers 拖拽排序属性 ─────────────
        if current_version < 9 {
            let _ = conn.execute_batch(
                "ALTER TABLE providers ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (9, datetime('now'));",
            )?;
        }

        // ── Migration 10: Adv. Pricing Engine ─────────────
        if current_version < 10 {
            let _ = conn.execute_batch(
                "ALTER TABLE token_usage_records ADD COLUMN total_cost_usd REAL NOT NULL DEFAULT 0.0;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (10, datetime('now'));",
            )?;
        }

        // ── Migration 11: Security Firewall UI (IP Logs & Rules) ─────────────
        if current_version < 11 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS ip_access_logs (
                    id              TEXT PRIMARY KEY,
                    ip_address      TEXT NOT NULL,
                    endpoint        TEXT NOT NULL,
                    token_id        TEXT,
                    action_taken    TEXT NOT NULL,
                    reason          TEXT,
                    created_at      INTEGER NOT NULL
                );
                
                CREATE TABLE IF NOT EXISTS ip_rules (
                    id              TEXT PRIMARY KEY,
                    ip_cidr         TEXT NOT NULL UNIQUE,
                    rule_type       TEXT NOT NULL,
                    notes           TEXT,
                    is_active       BOOLEAN NOT NULL DEFAULT 1,
                    created_at      INTEGER NOT NULL
                );",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (11, datetime('now'));",
            )?;
        }

        Ok(())
    }

    /// 执行不返回数据的 SQL 语句
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(sql, params)
    }

    /// 执行查询并返回单行数据
    pub fn query_row<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> SqlResult<T>
    where
        F: FnOnce(&rusqlite::Row<'_>) -> SqlResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql, params, f)
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

    // ============================================================
    // IdeAccount / 降维指纹池 专有 CRUD
    // ============================================================

    /// 根据归属平台查询当前全体存活账号
    pub fn get_active_ide_accounts(&self, origin_platform: &str) -> SqlResult<Vec<crate::models::IdeAccount>> {
        self.query_rows(
            "SELECT 
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type, 
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id, 
                dev_device_id, sqm_id, quota_json, created_at, updated_at, last_used, tags
             FROM ide_accounts 
             WHERE origin_platform = ? AND status = 'active' AND is_proxy_disabled = 0",
            &[&origin_platform],
            |row| {
                use chrono::{DateTime, Utc};
                use std::str::FromStr;
                let machine_id: Option<String> = row.get(10)?;
                let profile = if let Some(mid) = machine_id {
                    Some(crate::models::DeviceProfile {
                        machine_id: mid,
                        mac_machine_id: row.get(11)?,
                        dev_device_id: row.get(12)?,
                        sqm_id: row.get(13)?,
                    })
                } else {
                    None
                };

                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "forbidden" => crate::models::AccountStatus::Forbidden,
                    "rate_limited" => crate::models::AccountStatus::RateLimited,
                    "expired" => crate::models::AccountStatus::Expired,
                    _ => crate::models::AccountStatus::Active,
                };

                let tags_json: Option<String> = row.get(18).ok();
                let tags: Vec<String> = tags_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default();

                Ok(crate::models::IdeAccount {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    origin_platform: row.get(2)?,
                    token: crate::models::OAuthToken {
                        access_token: row.get(3)?,
                        refresh_token: row.get(4)?,
                        expires_in: row.get(5)?,
                        token_type: row.get(6)?,
                        updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(16)?).unwrap_or_else(|_| Utc::now()),
                    },
                    status,
                    disabled_reason: row.get(8)?,
                    is_proxy_disabled: row.get(9)?,
                    created_at: DateTime::<Utc>::from_str(&row.get::<_, String>(15)?).unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(16)?).unwrap_or_else(|_| Utc::now()),
                    last_used: DateTime::<Utc>::from_str(&row.get::<_, String>(17)?).unwrap_or_else(|_| Utc::now()),
                    device_profile: profile,
                    quota_json: row.get(14)?,
                    tags,
                })
            },
        )
    }

    /// 更新某账号在高并发环境下的实时健康红绿灯状态
    pub fn update_ide_account_status(&self, id: &str, status: crate::models::AccountStatus, reason: Option<&str>) -> SqlResult<usize> {
        let status_str = match status {
            crate::models::AccountStatus::Active => "active",
            crate::models::AccountStatus::Expired => "expired",
            crate::models::AccountStatus::Forbidden => "forbidden",
            crate::models::AccountStatus::RateLimited => "rate_limited",
            crate::models::AccountStatus::Unknown => "unknown",
        };
        self.execute(
            "UPDATE ide_accounts SET status = ?, disabled_reason = ?, updated_at = datetime('now') WHERE id = ?",
            &[&status_str, &reason, &id],
        )
    }

    /// 获取全体存库的指纹账号（用于展示到上帝盘）
    pub fn get_all_ide_accounts(&self) -> SqlResult<Vec<crate::models::IdeAccount>> {
        self.query_rows(
            "SELECT 
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type, 
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id, 
                dev_device_id, sqm_id, quota_json, created_at, updated_at, last_used, tags
             FROM ide_accounts ORDER BY created_at DESC",
            &[],
            |row| {
                use chrono::{DateTime, Utc};
                use std::str::FromStr;
                let machine_id: Option<String> = row.get(10)?;
                let profile = if let Some(mid) = machine_id {
                    Some(crate::models::DeviceProfile {
                        machine_id: mid,
                        mac_machine_id: row.get(11)?,
                        dev_device_id: row.get(12)?,
                        sqm_id: row.get(13)?,
                    })
                } else {
                    None
                };

                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "forbidden" => crate::models::AccountStatus::Forbidden,
                    "rate_limited" => crate::models::AccountStatus::RateLimited,
                    "expired" => crate::models::AccountStatus::Expired,
                    _ => crate::models::AccountStatus::Active,
                };

                let tags_json: Option<String> = row.get(18).ok();
                let tags: Vec<String> = tags_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default();

                Ok(crate::models::IdeAccount {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    origin_platform: row.get(2)?,
                    token: crate::models::OAuthToken {
                        access_token: row.get(3)?,
                        refresh_token: row.get(4)?,
                        expires_in: row.get(5)?,
                        token_type: row.get(6)?,
                        updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(16)?).unwrap_or_else(|_| Utc::now()),
                    },
                    status,
                    disabled_reason: row.get(8)?,
                    is_proxy_disabled: row.get(9)?,
                    created_at: DateTime::<Utc>::from_str(&row.get::<_, String>(15)?).unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(16)?).unwrap_or_else(|_| Utc::now()),
                    last_used: DateTime::<Utc>::from_str(&row.get::<_, String>(17)?).unwrap_or_else(|_| Utc::now()),
                    device_profile: profile,
                    quota_json: row.get(14)?,
                    tags,
                })
            },
        )
    }

    /// 插入或全量热更一个指纹账号（核心武器库导入接口）
    pub fn upsert_ide_account(&self, acc: &crate::models::IdeAccount) -> SqlResult<usize> {
        let status_str = match acc.status {
            crate::models::AccountStatus::Active => "active",
            crate::models::AccountStatus::Expired => "expired",
            crate::models::AccountStatus::Forbidden => "forbidden",
            crate::models::AccountStatus::RateLimited => "rate_limited",
            crate::models::AccountStatus::Unknown => "unknown",
        };
        let (mid, mac, did, sqm) = match &acc.device_profile {
            Some(p) => (Some(&p.machine_id), Some(&p.mac_machine_id), Some(&p.dev_device_id), Some(&p.sqm_id)),
            None => (None, None, None, None),
        };
        
        let c_at = acc.created_at.to_rfc3339();
        let u_at = acc.updated_at.to_rfc3339();
        let lu = acc.last_used.to_rfc3339();

        self.execute(
            "INSERT INTO ide_accounts (
                id, email, origin_platform, access_token, refresh_token, expires_in, token_type,
                status, disabled_reason, is_proxy_disabled, machine_id, mac_machine_id,
                dev_device_id, sqm_id, quota_json, created_at, updated_at, last_used
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(id) DO UPDATE SET 
                email = excluded.email,
                origin_platform = excluded.origin_platform,
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                expires_in = excluded.expires_in,
                token_type = excluded.token_type,
                status = excluded.status,
                disabled_reason = excluded.disabled_reason,
                is_proxy_disabled = excluded.is_proxy_disabled,
                machine_id = excluded.machine_id,
                mac_machine_id = excluded.mac_machine_id,
                dev_device_id = excluded.dev_device_id,
                sqm_id = excluded.sqm_id,
                quota_json = excluded.quota_json,
                updated_at = excluded.updated_at,
                last_used = excluded.last_used",
            rusqlite::params![
                acc.id, acc.email, acc.origin_platform, acc.token.access_token, acc.token.refresh_token,
                acc.token.expires_in, acc.token.token_type, status_str, acc.disabled_reason,
                acc.is_proxy_disabled, mid, mac, did, sqm, acc.quota_json, c_at, u_at, lu
            ],
        )
    }

    /// 删除僵尸账户
    pub fn delete_ide_account(&self, id: &str) -> SqlResult<usize> {
        self.execute("DELETE FROM ide_accounts WHERE id = ?", &[&id])
    }

    /// 执行自动恢复 Rate Limit (429) 限流节点的引擎方法 (限流冷却复活)
    pub fn recover_rate_limited_nodes(&self) {
        let conn = self.conn.lock().unwrap();
        // 复活超过 5 分钟的 API Keys
        let api_key_sql = "UPDATE api_keys
                           SET status = 'valid'
                           WHERE status = 'rate_limit'
                           AND last_checked_at < datetime('now', '-5 minutes');";
        
        let api_rows = conn.execute(api_key_sql, []).unwrap_or(0);
        if api_rows > 0 {
            tracing::info!("♻️ [流量重塑] 成功复活了 {} 个进入冷却期的 API Key 节点", api_rows);
        }

        // 复活超过 5 分钟的高优 IDE 僵尸账号
        let ide_account_sql = "UPDATE ide_accounts
                               SET status = 'active'
                               WHERE status = 'rate_limited'
                               AND updated_at < datetime('now', '-5 minutes');";
        
        let ide_rows = conn.execute(ide_account_sql, []).unwrap_or(0);
        if ide_rows > 0 {
            tracing::info!("♻️ [降维预警解除] 成功复活了 {} 个高优 IDE 伪装节点的 Rate Limit", ide_rows);
        }
    }

    /// 更新 IDE 账号标签（tags JSON 数组）
    pub fn update_ide_account_tags(&self, id: &str, tags_json: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE ide_accounts SET tags = ?, updated_at = datetime('now') WHERE id = ?",
            &[&tags_json, &id],
        )
    }

    /// 更新 API Key 标签
    pub fn update_api_key_tags(&self, id: &str, tags_json: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE api_keys SET tags = ? WHERE id = ?",
            &[&tags_json, &id],
        )
    }
}
