use crate::db::Database;
use crate::error::AppResult;
use crate::services::notify::{
    send_dingtalk, send_email, send_feishu, send_wecom, NotifyConfig,
};
use serde::{Deserialize, Serialize};
use tauri::State;

// ─────────────────────────────────────────────
// DTO（前端 ↔ 后端传输对象）
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyConfigDto {
    pub feishu_enabled: bool,
    pub feishu_webhook: String,

    pub dingtalk_enabled: bool,
    pub dingtalk_webhook: String,
    pub dingtalk_secret: String,

    pub wecom_enabled: bool,
    pub wecom_webhook: String,

    pub email_enabled: bool,
    pub email_smtp_host: String,
    pub email_smtp_port: u16,
    pub email_username: String,
    pub email_password: String,
    pub email_to: String,
}

impl From<NotifyConfig> for NotifyConfigDto {
    fn from(c: NotifyConfig) -> Self {
        Self {
            feishu_enabled: c.feishu_enabled,
            feishu_webhook: c.feishu_webhook,
            dingtalk_enabled: c.dingtalk_enabled,
            dingtalk_webhook: c.dingtalk_webhook,
            dingtalk_secret: c.dingtalk_secret,
            wecom_enabled: c.wecom_enabled,
            wecom_webhook: c.wecom_webhook,
            email_enabled: c.email_enabled,
            email_smtp_host: c.email_smtp_host,
            email_smtp_port: c.email_smtp_port,
            email_username: c.email_username,
            email_password: c.email_password,
            email_to: c.email_to,
        }
    }
}

// ─────────────────────────────────────────────
// Tauri 命令
// ─────────────────────────────────────────────

/// 读取所有 notify_ 配置
#[tauri::command]
pub async fn get_notify_config(db: State<'_, Database>) -> AppResult<NotifyConfigDto> {
    let cfg = NotifyConfig::load(&db);
    Ok(NotifyConfigDto::from(cfg))
}

/// 批量写入 notify_ 配置
#[tauri::command]
pub async fn save_notify_config(
    db: State<'_, Database>,
    config: NotifyConfigDto,
) -> AppResult<()> {
    let kvs: Vec<(String, String)> = vec![
        (
            "notify_feishu_enabled".to_string(),
            config.feishu_enabled.to_string(),
        ),
        (
            "notify_feishu_webhook".to_string(),
            config.feishu_webhook.clone(),
        ),
        (
            "notify_dingtalk_enabled".to_string(),
            config.dingtalk_enabled.to_string(),
        ),
        (
            "notify_dingtalk_webhook".to_string(),
            config.dingtalk_webhook.clone(),
        ),
        (
            "notify_dingtalk_secret".to_string(),
            config.dingtalk_secret.clone(),
        ),
        (
            "notify_wecom_enabled".to_string(),
            config.wecom_enabled.to_string(),
        ),
        (
            "notify_wecom_webhook".to_string(),
            config.wecom_webhook.clone(),
        ),
        (
            "notify_email_enabled".to_string(),
            config.email_enabled.to_string(),
        ),
        (
            "notify_email_smtp_host".to_string(),
            config.email_smtp_host.clone(),
        ),
        (
            "notify_email_smtp_port".to_string(),
            config.email_smtp_port.to_string(),
        ),
        (
            "notify_email_username".to_string(),
            config.email_username.clone(),
        ),
        (
            "notify_email_password".to_string(),
            config.email_password.clone(),
        ),
        ("notify_email_to".to_string(), config.email_to.clone()),
    ];

    db.set_account_settings_batch(&kvs)
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
    Ok(())
}

/// 测试单个通道（channel: "feishu" | "dingtalk" | "wecom" | "email"）
#[tauri::command]
pub async fn test_notify_channel(
    db: State<'_, Database>,
    channel: String,
    title: String,
    content: String,
) -> AppResult<()> {
    let cfg = NotifyConfig::load(&db);

    match channel.as_str() {
        "feishu" => {
            send_feishu(&cfg.feishu_webhook, &title, &content)
                .await
                .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
        }
        "dingtalk" => {
            let secret = if cfg.dingtalk_secret.is_empty() {
                None
            } else {
                Some(cfg.dingtalk_secret.as_str())
            };
            send_dingtalk(&cfg.dingtalk_webhook, secret, &title, &content)
                .await
                .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
        }
        "wecom" => {
            send_wecom(&cfg.wecom_webhook, &title, &content)
                .await
                .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
        }
        "email" => {
            send_email(
                &cfg.email_smtp_host,
                cfg.email_smtp_port,
                &cfg.email_username,
                &cfg.email_password,
                &cfg.email_to,
                &title,
                &content,
            )
            .await
            .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!(e)))?;
        }
        other => {
            return Err(crate::error::AppError::Other(anyhow::anyhow!(
                "未知通道: {}",
                other
            )));
        }
    }

    Ok(())
}
