use crate::db::Database;
use crate::models::{Platform, TokenScope};
use crate::store::SecureStore;
use chrono::Utc;
use std::sync::Arc;

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

    /// 根据 Scope 和可选强制平台，挑选全网甚至细分池里最优的一个节点。
    /// 这里统一检索 api_keys 表与 ide_accounts 混合池。
    pub fn pick_best_key(
        &self,
        force_platform: Option<&str>,
        scope: &TokenScope,
    ) -> Option<RouteTarget> {
        let mut where_api_keys = vec!["status = 'valid'".to_string()];
        let mut where_ide_accs = vec!["status = 'active'".to_string()];

        // 1. 如果代码层级强制指定了某平台（例如客户端非要用 openai 接口），兜底过滤
        if let Some(p) = force_platform {
            where_api_keys.push(format!("platform = '{}'", p));
            where_ide_accs.push(format!("origin_platform = '{}'", p));
        }

        // 2. 根据前端传入的分发规则切片漏斗
        match scope.scope.as_str() {
            "single" => {
                if let Some(target_id) = &scope.single_account {
                    let sanitized = target_id.replace("'", "''");
                    where_api_keys.push(format!("id = '{}'", sanitized));
                    where_ide_accs.push(format!("id = '{}'", sanitized));
                } else {
                    return None; // 单点透传缺少目标的直接拦截
                }
            }
            "channel" => {
                let mut channel_ins_api = vec![];
                let mut channel_ins_ide = vec![];
                for ch in &scope.channels {
                    if let Some(plat) = ch.strip_prefix("api_") {
                        channel_ins_api.push(format!("'{}'", plat.replace("'", "''")));
                    } else if let Some(plat) = ch.strip_prefix("ide_") {
                        channel_ins_ide.push(format!("'{}'", plat.replace("'", "''")));
                    }
                }

                // 如果 channel 列表里啥都没有当前分类的，强造一条无法命中的规则
                if channel_ins_api.is_empty() {
                    where_api_keys.push("1=0".to_string());
                } else {
                    where_api_keys.push(format!("platform IN ({})", channel_ins_api.join(",")));
                }

                if channel_ins_ide.is_empty() {
                    where_ide_accs.push("1=0".to_string());
                } else {
                    where_ide_accs.push(format!(
                        "origin_platform IN ({})",
                        channel_ins_ide.join(",")
                    ));
                }
            }
            "tag" => {
                if scope.tags.is_empty() {
                    // 没有标签直接拦截
                    where_api_keys.push("1=0".to_string());
                    where_ide_accs.push("1=0".to_string());
                } else {
                    // 使用 LIKE 来做简陋的 JSON 数组包含过滤。因为 Sqlite 原生 json 函数麻烦
                    let mut api_opts = vec![];
                    let mut ide_opts = vec![];
                    for t in &scope.tags {
                        let t_escaped = t.replace("'", "''");
                        api_opts.push(format!("tags LIKE '%\"{}\"%'", t_escaped));
                        ide_opts.push(format!("tags LIKE '%\"{}\"%'", t_escaped));
                    }
                    where_api_keys.push(format!("({})", api_opts.join(" OR ")));
                    where_ide_accs.push(format!("({})", ide_opts.join(" OR ")));
                }
            }
            "global" | _ => {
                // 不受限，全流开放漫游，不追加任何 where
            }
        }

        let api_where = where_api_keys.join(" AND ");
        let ide_where = where_ide_accs.join(" AND ");

        // 读取由高级控制板投递的引擎配置
        let order_clause = if let Ok(engine_cfg) = crate::commands::proxy::ENGINE_CONFIG.read() {
            if engine_cfg.scheduling.mode.eq_ignore_ascii_case("balance") {
                // 轮询模式：优先挑选最近没怎么使用的
                "ORDER BY last_checked_at ASC"
            } else {
                // 优先级模式 或其他：优先级为主，最近没使用的次之
                "ORDER BY priority DESC, last_checked_at ASC"
            }
        } else {
            "ORDER BY priority DESC, last_checked_at DESC"
        };

        // 终极聚合 SQL，将两个异构数据源统一为 标准 RouteTarget 模型输出
        // id | source_type | platform | base_url | access_token | priority
        let sql = format!(
            "SELECT id, 'api_key' AS source_type, platform, base_url, '' AS access_token, priority, last_checked_at 
             FROM api_keys WHERE {}
             UNION ALL
             SELECT id, 'ide_account' AS source_type, origin_platform AS platform, '' AS base_url, access_token, 80 AS priority, last_used AS last_checked_at
             FROM ide_accounts WHERE {}
             {} LIMIT 1",
            api_where, ide_where, order_clause
        );

        let result_row: Option<(String, String, String, Option<String>, String)> = self
            .db
            .query_one(&sql, &[], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .ok();

        if let Some((id, source_type, platform_str, base_url, access_token)) = result_row {
            let platform = serde_json::from_str::<Platform>(&format!("\"{}\"", platform_str))
                .unwrap_or(Platform::Custom);

            let secret = if source_type == "api_key" {
                SecureStore::get_key(&id).ok()?
            } else {
                access_token
            };

            Some(RouteTarget {
                key_id: id,
                secret,
                platform,
                base_url,
            })
        } else {
            None
        }
    }

    /// 标记节点死亡状态（由于现在是混合池，所以需要判断 ID 在哪张表里）
    pub fn mark_key_status(&self, key_id: &str, is_ide_account: bool, status: &str) {
        if is_ide_account {
            let _ = self.db.execute(
                "UPDATE ide_accounts SET status = ?1, updated_at = ?2 WHERE id = ?3",
                &[&status, &Utc::now().to_rfc3339(), &key_id],
            );
        } else {
            let _ = self.db.execute(
                "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
                &[&status, &Utc::now().to_rfc3339(), &key_id],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::Path;

    /// 构造一个跑完 migrations 的内存库，供路由测试使用
    fn make_db() -> Arc<Database> {
        let db = Database::new(Path::new(":memory:")).expect("open in-memory db");
        Arc::new(db)
    }

    fn insert_ide_account(
        db: &Database,
        id: &str,
        platform: &str,
        status: &str,
        access_token: &str,
        last_used: &str,
        tags: Option<&str>,
    ) {
        let now = "2026-01-01T00:00:00Z";
        let tags_sql = tags.unwrap_or("[]");
        db.execute(
            "INSERT INTO ide_accounts \
             (id, email, origin_platform, access_token, refresh_token, expires_in, token_type, \
              status, is_proxy_disabled, created_at, updated_at, last_used, tags) \
             VALUES (?1, ?2, ?3, ?4, '', 0, 'Bearer', ?5, 0, ?6, ?6, ?7, ?8)",
            &[&id, &"x@example.com", &platform, &access_token, &status, &now, &last_used, &tags_sql],
        )
        .expect("insert ide_account");
    }

    fn scope(name: &str) -> TokenScope {
        TokenScope {
            scope: name.to_string(),
            desc: None,
            channels: vec![],
            tags: vec![],
            single_account: None,
        }
    }

    #[test]
    fn global_scope_picks_least_recently_used_active_account() {
        let db = make_db();
        // 默认调度模式（priority 模式）：ORDER BY priority DESC, last_checked_at ASC
        // 即优先选"最久没用的"账号，实现负载均衡。
        // 两条 ide_account 优先级均为 80，因此由 last_used ASC 决定 — 最久没用的优先。
        insert_ide_account(&db, "a", "anthropic", "active", "tok-a", "2026-01-01T10:00:00Z", None);
        insert_ide_account(&db, "b", "anthropic", "active", "tok-b", "2026-01-02T10:00:00Z", None);

        let target = Router::new(db).pick_best_key(None, &scope("global")).expect("有结果");
        // a 的 last_used 更旧（2026-01-01 < 2026-01-02），在 priority 模式下应当被优先选中
        assert_eq!(target.key_id, "a", "应当挑 last_used 更旧的账号（最久没用的优先）");
        assert_eq!(target.secret, "tok-a");
    }

    #[test]
    fn global_scope_skips_inactive_accounts() {
        let db = make_db();
        insert_ide_account(&db, "dead", "anthropic", "forbidden", "tok-dead", "2026-01-03T00:00:00Z", None);
        insert_ide_account(&db, "alive", "anthropic", "active", "tok-alive", "2026-01-01T00:00:00Z", None);

        let target = Router::new(db).pick_best_key(None, &scope("global")).expect("有结果");
        assert_eq!(target.key_id, "alive");
    }

    #[test]
    fn single_scope_returns_targeted_account() {
        let db = make_db();
        insert_ide_account(&db, "a", "anthropic", "active", "tok-a", "2026-01-01T00:00:00Z", None);
        insert_ide_account(&db, "b", "anthropic", "active", "tok-b", "2026-01-02T00:00:00Z", None);

        let mut s = scope("single");
        s.single_account = Some("a".to_string());

        let target = Router::new(db).pick_best_key(None, &s).expect("有结果");
        assert_eq!(target.key_id, "a");
    }

    #[test]
    fn single_scope_returns_none_without_target_id() {
        let db = make_db();
        insert_ide_account(&db, "a", "anthropic", "active", "tok-a", "2026-01-01T00:00:00Z", None);

        // single 但没指定 single_account，应该被拦截返回 None
        let s = scope("single");
        assert!(Router::new(db).pick_best_key(None, &s).is_none());
    }

    #[test]
    fn channel_scope_filters_by_ide_prefix() {
        let db = make_db();
        insert_ide_account(&db, "claude", "anthropic", "active", "tok-c", "2026-01-01T00:00:00Z", None);
        insert_ide_account(&db, "gem", "gemini", "active", "tok-g", "2026-01-02T00:00:00Z", None);

        let mut s = scope("channel");
        s.channels = vec!["ide_anthropic".to_string()];

        let target = Router::new(db).pick_best_key(None, &s).expect("有结果");
        assert_eq!(target.key_id, "claude");
    }

    #[test]
    fn channel_scope_with_only_api_channels_returns_none_for_ide_only_pool() {
        let db = make_db();
        insert_ide_account(&db, "claude", "anthropic", "active", "tok-c", "2026-01-01T00:00:00Z", None);

        // 只允许 api_anthropic 类型，但池子里只有 ide_account → 应找不到
        let mut s = scope("channel");
        s.channels = vec!["api_anthropic".to_string()];

        assert!(Router::new(db).pick_best_key(None, &s).is_none());
    }

    #[test]
    fn tag_scope_filters_by_tag_in_json_array() {
        let db = make_db();
        insert_ide_account(&db, "vip", "anthropic", "active", "tok-vip", "2026-01-01T00:00:00Z", Some(r#"["pro","ultra"]"#));
        insert_ide_account(&db, "norm", "anthropic", "active", "tok-norm", "2026-01-02T00:00:00Z", Some(r#"["free"]"#));

        let mut s = scope("tag");
        s.tags = vec!["ultra".to_string()];

        let target = Router::new(db).pick_best_key(None, &s).expect("有结果");
        assert_eq!(target.key_id, "vip");
    }

    #[test]
    fn tag_scope_with_empty_tags_returns_none() {
        let db = make_db();
        insert_ide_account(&db, "vip", "anthropic", "active", "tok-vip", "2026-01-01T00:00:00Z", Some(r#"["pro"]"#));

        // tag scope 但 tags 列表为空 → 拦截
        let s = scope("tag");
        assert!(Router::new(db).pick_best_key(None, &s).is_none());
    }

    #[test]
    fn force_platform_overrides_pool_when_mismatched() {
        let db = make_db();
        insert_ide_account(&db, "claude", "anthropic", "active", "tok-c", "2026-01-01T00:00:00Z", None);

        // 强制 gemini，但池子里只有 anthropic → 找不到
        assert!(Router::new(db.clone()).pick_best_key(Some("gemini"), &scope("global")).is_none());
        // 强制 anthropic 与池子一致 → 命中
        let target = Router::new(db).pick_best_key(Some("anthropic"), &scope("global")).expect("有结果");
        assert_eq!(target.key_id, "claude");
    }

    #[test]
    fn mark_key_status_writes_back_for_ide_account() {
        let db = make_db();
        insert_ide_account(&db, "a", "anthropic", "active", "tok-a", "2026-01-01T00:00:00Z", None);

        Router::new(db.clone()).mark_key_status("a", true, "rate_limited");

        let status: String = db
            .query_one("SELECT status FROM ide_accounts WHERE id = ?1", &[&"a"], |r| r.get(0))
            .expect("读取状态");
        assert_eq!(status, "rate_limited");
    }
}
