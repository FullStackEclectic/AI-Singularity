use crate::db::Database;
use crate::store::SecureStore;
use chrono::Utc;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub struct HealthCheckDaemon {
    db: Arc<Database>,
    client: Client,
}

impl HealthCheckDaemon {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 在后台作为独立任务启动
    pub fn start(self, interval_minutes: u64) {
        tokio::spawn(async move {
            tracing::info!(
                "守护进程已启动：API Key 主动健康探活 (每 {} 分钟执行一次)",
                interval_minutes
            );
            let mut interval = time::interval(Duration::from_secs(interval_minutes * 60));
            // 第一次启动时会立刻触发（interval的特性），我们可以跳过第一次如果需要
            interval.tick().await;

            loop {
                interval.tick().await;
                tracing::info!("开始执行全网点自动健康探活...");
                self.run_checks().await;
                tracing::info!("全网点健康探活任务执行完毕。");
            }
        });
    }

    async fn run_checks(&self) {
        let keys_to_test: Vec<(String, String, Option<String>, String)> = self
            .db
            .query_rows("SELECT id, platform, base_url, status FROM api_keys", &[], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })
            .unwrap_or_default();

        for (id, platform_str, base_url, status) in keys_to_test {
            let secret = match SecureStore::get_key(&id) {
                Ok(s) => s,
                Err(_) => {
                    self.update_status(&id, "invalid");
                    continue;
                }
            };

            // 构建极其廉价的请求，只为探活 401
            let is_alive = self.ping_api(&platform_str, base_url.as_deref(), &secret).await;

            let new_status = if is_alive { "valid" } else { "invalid" };

            if status != new_status {
                tracing::warn!("探活状态变更: Key[{}] {} -> {}", id, status, new_status);
                self.update_status(&id, new_status);
                // 这里未来可通过 channel 或 IPC 抛送 Desktop 通知
            } else {
                // 如果只是成功测活但状态没变，仅更新 last_checked_at
                let _ = self.db.execute(
                    "UPDATE api_keys SET last_checked_at = ?1 WHERE id = ?2",
                    &[&Utc::now().to_rfc3339(), &id],
                );
            }
        }
    }

    async fn ping_api(&self, platform_str: &str, base_url: Option<&str>, secret: &str) -> bool {
        // 由于只是测 401 封禁情况，最经济的做法是调用 /v1/models (完全免费且不消耗Token)
        // 注意：这里去掉了多余的双引号匹配，使用正确的小写匹配
        let endpoint = if let Some(base) = base_url {
            let base = base.trim_end_matches('/');
            format!("{}/v1/models", base)
        } else {
            match platform_str {
                "openai" => "https://api.openai.com/v1/models".to_string(),
                "anthropic" => "https://api.anthropic.com/v1/models".to_string(),
                "gemini" => format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", secret),
                "deep_seek" => "https://api.deepseek.com/v1/models".to_string(),
                "aliyun" => "https://dashscope.aliyuncs.com/compatible-mode/v1/models".to_string(),
                _ => return true, // 未知平台默认乐观认为存活
            }
        };

        if platform_str == "anthropic" {
            // Anthropic 没有公用的 /v1/models，发送无内容鉴权包
            let resp = self
                .client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", secret)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": "claude-3-haiku-20240307",
                    "max_tokens": 1,
                    "messages": [{"role":"user","content":"Hi"}]
                }))
                .send()
                .await;
            if let Ok(res) = resp {
                if res.status().is_client_error() {
                    return res.status().as_u16() == 429; // 429是过载，不算死Key
                }
                return true;
            }
            return false;
        }

        // 以 OpenAI 兼容规范发起测速
        let resp = self
            .client
            .get(&endpoint)
            .header("Authorization", format!("Bearer {}", secret))
            .send()
            .await;

        match resp {
            Ok(res) => {
                let status = res.status();
                // 只要未因鉴权打回 401 / 被 WAF 封锁 403，都当做可用线路
                status.as_u16() != 401 && status.as_u16() != 403
            }
            Err(_) => false,
        }
    }

    fn update_status(&self, id: &str, status: &str) {
        let _ = self.db.execute(
            "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
            &[&status, &Utc::now().to_rfc3339(), &id],
        );
    }
}
