use super::Database;
use rusqlite::Result as SqlResult;

impl Database {
    pub(super) fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

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

        if current_version < 2 {
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
            let _ = conn.execute_batch("ALTER TABLE prompts ADD COLUMN description TEXT;");
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

        if current_version < 6 {
            let _ = conn.execute_batch(
                "ALTER TABLE api_keys ADD COLUMN priority INTEGER NOT NULL DEFAULT 100;",
            );
            let _ = conn.execute_batch("ALTER TABLE prompts ADD COLUMN tool_targets TEXT;");
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (6, datetime('now'));",
            )?;
        }

        if current_version < 7 {
            let _ = conn.execute_batch(
                "ALTER TABLE ide_accounts ADD COLUMN tags TEXT;
                 ALTER TABLE api_keys ADD COLUMN tags TEXT;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (7, datetime('now'));",
            )?;
        }

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

        if current_version < 9 {
            let _ = conn.execute_batch(
                "ALTER TABLE providers ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (9, datetime('now'));",
            )?;
        }

        if current_version < 10 {
            let _ = conn.execute_batch(
                "ALTER TABLE token_usage_records ADD COLUMN total_cost_usd REAL NOT NULL DEFAULT 0.0;",
            );
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (10, datetime('now'));",
            )?;
        }

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

        if current_version < 12 {
            let _ = conn.execute_batch("ALTER TABLE ide_accounts ADD COLUMN project_id TEXT;");
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (12, datetime('now'));",
            )?;
        }

        if current_version < 13 {
            let _ = conn.execute_batch("ALTER TABLE ide_accounts ADD COLUMN meta_json TEXT;");
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (13, datetime('now'));",
            )?;
        }

        if current_version < 14 {
            let _ = conn.execute_batch("ALTER TABLE ide_accounts ADD COLUMN label TEXT;");
            conn.execute_batch(
                "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (14, datetime('now'));",
            )?;
        }

        Ok(())
    }
}
