use crate::db::Database;
use crate::models::{AlertItem, AlertLevel};
use chrono::{Duration, Utc};
use uuid::Uuid;

pub struct AlertService<'a> {
    db: &'a Database,
}

impl<'a> AlertService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 扫描所有潜在告警：Key 失效、过期、余额不足
    pub fn get_alerts(&self) -> Vec<AlertItem> {
        let mut alerts = Vec::new();

        // 1. 检查 Key 状态告警
        self.check_key_alerts(&mut alerts);

        // 2. 检查余额低告警
        self.check_balance_alerts(&mut alerts);

        alerts
    }

    fn check_key_alerts(&self, alerts: &mut Vec<AlertItem>) {
        let sql = "SELECT id, name, platform, status, last_checked_at FROM api_keys";
        let rows = self.db.query_rows(sql, &[], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let platform: String = row.get(2)?;
            let status: String = row.get(3)?;
            let last_checked: Option<String> = row.get(4)?;
            Ok((id, name, platform, status, last_checked))
        });

        if let Ok(keys) = rows {
            for (id, name, platform, status, last_checked) in keys {
                match status.as_str() {
                    "invalid" => {
                        alerts.push(AlertItem {
                            id: Uuid::new_v4().to_string(),
                            level: AlertLevel::Critical,
                            title: format!("Key 无效: {}", name),
                            message: format!(
                                "{} 的 API Key「{}」已失效，请及时更新",
                                platform, name
                            ),
                            platform: Some(platform),
                            key_id: Some(id),
                        });
                    }
                    "banned" => {
                        alerts.push(AlertItem {
                            id: Uuid::new_v4().to_string(),
                            level: AlertLevel::Critical,
                            title: format!("Key 被封禁: {}", name),
                            message: format!("{} 的 API Key「{}」已被封禁 (403)", platform, name),
                            platform: Some(platform),
                            key_id: Some(id),
                        });
                    }
                    "expired" => {
                        alerts.push(AlertItem {
                            id: Uuid::new_v4().to_string(),
                            level: AlertLevel::Warning,
                            title: format!("Key 已过期: {}", name),
                            message: format!(
                                "{} 的 API Key「{}」已过期，请重新申请",
                                platform, name
                            ),
                            platform: Some(platform),
                            key_id: Some(id),
                        });
                    }
                    "unknown" => {
                        // 超过 24h 未检测也提示
                        if let Some(ts) = last_checked {
                            if let Ok(last) = ts.parse::<chrono::DateTime<Utc>>() {
                                if Utc::now() - last > Duration::hours(24) {
                                    alerts.push(AlertItem {
                                        id: Uuid::new_v4().to_string(),
                                        level: AlertLevel::Info,
                                        title: format!("Key 待检测: {}", name),
                                        message: format!(
                                            "{} 的 Key「{}」已超过 24 小时未检测状态",
                                            platform, name
                                        ),
                                        platform: Some(platform),
                                        key_id: Some(id),
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_balance_alerts(&self, alerts: &mut Vec<AlertItem>) {
        // 余额低于 $1 / ¥5 时触发告警
        let sql = "SELECT key_id, platform, balance_usd, balance_cny FROM balances";
        let rows = self.db.query_rows(sql, &[], |row| {
            let key_id: String = row.get(0)?;
            let platform: String = row.get(1)?;
            let balance_usd: Option<f64> = row.get(2)?;
            let balance_cny: Option<f64> = row.get(3)?;
            Ok((key_id, platform, balance_usd, balance_cny))
        });

        if let Ok(balances) = rows {
            for (key_id, platform, balance_usd, balance_cny) in balances {
                let low_usd = balance_usd.map(|b| b < 1.0).unwrap_or(false);
                let low_cny = balance_cny.map(|b| b < 5.0).unwrap_or(false);

                if low_usd || low_cny {
                    let amount = balance_usd
                        .map(|b| format!("${:.2}", b))
                        .or_else(|| balance_cny.map(|b| format!("¥{:.2}", b)))
                        .unwrap_or_else(|| "极低".to_string());

                    alerts.push(AlertItem {
                        id: Uuid::new_v4().to_string(),
                        level: AlertLevel::Warning,
                        title: format!("{} 余额不足", platform),
                        message: format!(
                            "{} 账户余额仅剩 {}，请及时充值以避免服务中断",
                            platform, amount
                        ),
                        platform: Some(platform),
                        key_id: Some(key_id),
                    });
                }
            }
        }
    }

    /// 发送 OS 原生通知（带频率控制，同一条 Alert 24 小时内只推一次）
    pub fn notify_os_throttle(&self, app: &tauri::AppHandle, alerts: Vec<AlertItem>) {
        use tauri_plugin_notification::NotificationExt;

        for alert in alerts {
            let should_notify = match alert.level {
                AlertLevel::Critical | AlertLevel::Warning => true,
                _ => false,
            };

            if should_notify {
                // 生成一个不基于 UUID 而是基于内容/Key的确定性 ID，防止每次刷新 UUID 都变导致风暴轰炸
                let stable_id = if let Some(k) = &alert.key_id {
                    format!("{}_{}", k, alert.title.replace(" ", "_"))
                } else {
                    continue; // 无法确定身份的告警跳过自动推送
                };

                // 检查最后发送时间过滤
                let sql = "SELECT last_sent_at FROM alert_history WHERE alert_id = ?1";
                if let Ok(ts_str) =
                    self.db
                        .query_row(sql, rusqlite::params![&stable_id], |r: &rusqlite::Row| {
                            r.get::<_, String>(0)
                        })
                {
                    if let Ok(last_sent) = ts_str.parse::<chrono::DateTime<Utc>>() {
                        if Utc::now() - last_sent < Duration::hours(24) {
                            continue; // 24小时内不重复轰炸
                        }
                    }
                }

                // 发送 OS 通知
                let title = alert.title.clone();
                let msg = alert.message.clone();
                let _ = app.notification().builder().title(&title).body(&msg).show();

                tracing::warn!("📳 OS 高危弹窗已推送: {} - {}", title, msg);

                // 记录已发送
                let _ = self.db.execute(
                    "INSERT OR REPLACE INTO alert_history (alert_id, last_sent_at) VALUES (?1, ?2)",
                    rusqlite::params![&stable_id, &Utc::now().to_rfc3339()],
                );
            }
        }
    }
}
