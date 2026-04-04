use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 巨型高并发本地账号池智能轮询引擎 (Account Pool Router)
pub struct AccountPoolManager {
    db: Arc<Database>,
    /// 游标缓存池 (platform -> index)
    cursors: RwLock<HashMap<String, usize>>,
}

impl AccountPoolManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            cursors: RwLock::new(HashMap::new()),
        }
    }

    /// 轮询池寻址算法：获取特定应用平台（如 ClaudeCode）下的下一个幸存伪装账号
    /// 采用了无锁弱同步的设计（通过定期刷新 DB）保证即使某一条线程把号用死了，下一个请求也能摘清历史
    pub fn get_next_available_account(&self, origin_platform: &str) -> Option<IdeAccount> {
        let accounts = self
            .db
            .get_active_ide_accounts(origin_platform)
            .unwrap_or_default();

        if accounts.is_empty() {
            return None;
        }

        let mut cursors = self.cursors.write().unwrap();
        let cursor = cursors.entry(origin_platform.to_string()).or_insert(0);

        // Round Robin 无限火力轮询
        *cursor = (*cursor + 1) % accounts.len();
        let selected = accounts[*cursor].clone();

        tracing::info!(
            "🔄 [账号池调度] {} 命中轮询序列号 {}, 提拔底层凭证: {}",
            origin_platform,
            *cursor,
            selected.email
        );

        Some(selected)
    }

    /// 触发 403 熔断保护：当任意下位客户端触发原站 403 惩罚时，直接将该账号从池子中摘除！
    pub fn report_account_dead(&self, id: &str, reason: &str) {
        let _ = self
            .db
            .update_ide_account_status(id, AccountStatus::Forbidden, Some(reason));
        tracing::error!(
            "🔥 [降维预警] 发现高优轮询节点遭到目标平台流控绞杀 [ID: {}]! 原因: {}. 该实体已被无情抛弃，底层路由池已抹除此节点。",
            id,
            reason
        );
    }

    /// 触发 429 限流保护：标记该账号受限，稍后可复活
    pub fn report_account_rate_limited(&self, id: &str) {
        let _ = self
            .db
            .update_ide_account_status(id, AccountStatus::RateLimited, Some("Rate limited (429)"));
        tracing::warn!(
            "⚠️ [流量规避] 节点 [ID: {}] 遭到 429 限流，已标记受限，进入冷却周期。",
            id
        );
    }
}
