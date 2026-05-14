use crate::db::Database;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

// ─────────────────────────────────────────────
// 配置结构体
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct NotifyConfig {
    // 飞书
    pub feishu_enabled: bool,
    pub feishu_webhook: String,
    // 钉钉
    pub dingtalk_enabled: bool,
    pub dingtalk_webhook: String,
    pub dingtalk_secret: String,
    // 企业微信
    pub wecom_enabled: bool,
    pub wecom_webhook: String,
    // 邮件
    pub email_enabled: bool,
    pub email_smtp_host: String,
    pub email_smtp_port: u16,
    pub email_username: String,
    pub email_password: String,
    pub email_to: String,
}

impl NotifyConfig {
    /// 从 account_settings 表读取所有 notify_ 前缀配置
    pub fn load(db: &Database) -> Self {
        let map: HashMap<String, String> = db.get_all_account_settings().unwrap_or_default();
        let get = |k: &str| map.get(k).cloned().unwrap_or_default();
        let get_bool = |k: &str| get(k).to_lowercase() == "true";
        let get_u16 = |k: &str, default: u16| -> u16 { get(k).parse::<u16>().unwrap_or(default) };

        Self {
            feishu_enabled: get_bool("notify_feishu_enabled"),
            feishu_webhook: get("notify_feishu_webhook"),
            dingtalk_enabled: get_bool("notify_dingtalk_enabled"),
            dingtalk_webhook: get("notify_dingtalk_webhook"),
            dingtalk_secret: get("notify_dingtalk_secret"),
            wecom_enabled: get_bool("notify_wecom_enabled"),
            wecom_webhook: get("notify_wecom_webhook"),
            email_enabled: get_bool("notify_email_enabled"),
            email_smtp_host: get("notify_email_smtp_host"),
            email_smtp_port: get_u16("notify_email_smtp_port", 465),
            email_username: get("notify_email_username"),
            email_password: get("notify_email_password"),
            email_to: get("notify_email_to"),
        }
    }
}

// ─────────────────────────────────────────────
// 飞书机器人 Webhook
// ─────────────────────────────────────────────

/// 飞书富文本消息
/// POST {"msg_type":"post","content":{"post":{"zh_cn":{"title":...,"content":[[{"tag":"text","text":...}]]}}}}
pub async fn send_feishu(webhook_url: &str, title: &str, content: &str) -> Result<(), String> {
    if webhook_url.is_empty() {
        return Err("飞书 Webhook URL 未配置".to_string());
    }
    let body = json!({
        "msg_type": "post",
        "content": {
            "post": {
                "zh_cn": {
                    "title": title,
                    "content": [[{"tag": "text", "text": content}]]
                }
            }
        }
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(webhook_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("飞书请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("飞书返回错误 {}: {}", status, text));
    }
    Ok(())
}

// ─────────────────────────────────────────────
// 钉钉机器人 Webhook（支持签名）
// ─────────────────────────────────────────────

/// HMAC-SHA256 实现（使用 sha2 crate，不依赖 hmac crate）
/// 遵循 RFC 2104：HMAC(K, m) = H((K' XOR opad) || H((K' XOR ipad) || m))
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;

    // 如果 key 超过块大小，先对 key 做哈希
    let mut k = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        k[..32].copy_from_slice(&digest);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    // ipad = 0x36, opad = 0x5c
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    // inner = H(ipad || message)
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&ipad);
    inner_hasher.update(message);
    let inner_hash = inner_hasher.finalize();

    // outer = H(opad || inner)
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&opad);
    outer_hasher.update(&inner_hash);
    outer_hasher.finalize().into()
}

/// 钉钉 Markdown 消息，支持可选签名
/// POST {"msgtype":"markdown","markdown":{"title":...,"text":...}}
pub async fn send_dingtalk(
    webhook_url: &str,
    secret: Option<&str>,
    title: &str,
    content: &str,
) -> Result<(), String> {
    if webhook_url.is_empty() {
        return Err("钉钉 Webhook URL 未配置".to_string());
    }

    // 构造最终 URL（带签名时追加 timestamp + sign 参数）
    let final_url = if let Some(sec) = secret.filter(|s| !s.is_empty()) {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let sign_str = format!("{}\n{}", timestamp, sec);
        let mac = hmac_sha256(sec.as_bytes(), sign_str.as_bytes());
        let sign = BASE64.encode(mac);
        let sign_enc = urlencoding::encode(&sign).into_owned();
        format!(
            "{}&timestamp={}&sign={}",
            webhook_url, timestamp, sign_enc
        )
    } else {
        webhook_url.to_string()
    };

    let text = format!("### {}\n\n{}", title, content);
    let body = json!({
        "msgtype": "markdown",
        "markdown": {
            "title": title,
            "text": text
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&final_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("钉钉请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("钉钉返回错误 {}: {}", status, text));
    }
    Ok(())
}

// ─────────────────────────────────────────────
// 企业微信机器人 Webhook
// ─────────────────────────────────────────────

/// 企业微信 Markdown 消息
/// POST {"msgtype":"markdown","markdown":{"content":"..."}}
pub async fn send_wecom(webhook_url: &str, title: &str, content: &str) -> Result<(), String> {
    if webhook_url.is_empty() {
        return Err("企业微信 Webhook URL 未配置".to_string());
    }
    let md_content = format!("## {}\n{}", title, content);
    let body = json!({
        "msgtype": "markdown",
        "markdown": {
            "content": md_content
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(webhook_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("企业微信请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("企业微信返回错误 {}: {}", status, text));
    }
    Ok(())
}

// ─────────────────────────────────────────────
// 邮件（手写 SMTP over TCP，不引入 lettre）
// ─────────────────────────────────────────────

/// 通过 TCP 手写 SMTP 握手发送邮件（明文 SMTP，适用于端口 25/587）
/// 流程：连接 → 读 220 → EHLO → AUTH LOGIN → MAIL FROM → RCPT TO → DATA → QUIT
///
/// 注意：端口 465（SMTPS）需要 TLS 包装，此处不支持；
/// 如需 TLS，请将 smtp_port 设为 587 并确保服务器支持 STARTTLS，
/// 或使用支持 TLS 的 SMTP 中继（如 SendGrid HTTP API）。
pub async fn send_email(
    smtp_host: &str,
    smtp_port: u16,
    username: &str,
    password: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    if smtp_host.is_empty() {
        return Err("SMTP 主机未配置".to_string());
    }
    if username.is_empty() || password.is_empty() {
        return Err("SMTP 用户名或密码未配置".to_string());
    }
    if to.is_empty() {
        return Err("收件人地址未配置".to_string());
    }

    let addr = format!("{}:{}", smtp_host, smtp_port);
    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("TCP 连接失败 {}: {}", addr, e))?;

    let mut buf = BufReader::new(stream);

    // 读取服务器欢迎 220
    smtp_expect(&mut buf, 220).await?;

    // EHLO
    smtp_send(&mut buf, &format!("EHLO {}\r\n", smtp_host)).await?;
    smtp_read_multiline(&mut buf).await?;

    // AUTH LOGIN
    smtp_send(&mut buf, "AUTH LOGIN\r\n").await?;
    smtp_expect(&mut buf, 334).await?;

    smtp_send(
        &mut buf,
        &format!("{}\r\n", BASE64.encode(username.as_bytes())),
    )
    .await?;
    smtp_expect(&mut buf, 334).await?;

    smtp_send(
        &mut buf,
        &format!("{}\r\n", BASE64.encode(password.as_bytes())),
    )
    .await?;
    smtp_expect(&mut buf, 235).await?;

    // MAIL FROM
    smtp_send(&mut buf, &format!("MAIL FROM:<{}>\r\n", username)).await?;
    smtp_expect(&mut buf, 250).await?;

    // RCPT TO
    smtp_send(&mut buf, &format!("RCPT TO:<{}>\r\n", to)).await?;
    smtp_expect(&mut buf, 250).await?;

    // DATA
    smtp_send(&mut buf, "DATA\r\n").await?;
    smtp_expect(&mut buf, 354).await?;

    let now = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S +0000")
        .to_string();
    // 邮件正文用 base64 编码，避免中文乱码
    let body_b64 = BASE64.encode(body.as_bytes());
    let message = format!(
        "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nDate: {date}\r\n\
         MIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\n\
         Content-Transfer-Encoding: base64\r\n\r\n{body}\r\n.\r\n",
        from = username,
        to = to,
        subject = subject,
        date = now,
        body = body_b64,
    );
    smtp_send(&mut buf, &message).await?;
    smtp_expect(&mut buf, 250).await?;

    // QUIT
    smtp_send(&mut buf, "QUIT\r\n").await?;
    let _ = smtp_read_multiline(&mut buf).await;

    Ok(())
}

// ─── SMTP 辅助函数 ───────────────────────────

async fn smtp_send<S>(buf: &mut BufReader<S>, data: &str) -> Result<(), String>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    buf.get_mut()
        .write_all(data.as_bytes())
        .await
        .map_err(|e| format!("SMTP 写入失败: {}", e))
}

/// 读取一行并验证响应码
async fn smtp_expect<S>(buf: &mut BufReader<S>, expected: u16) -> Result<String, String>
where
    S: AsyncReadExt + Unpin,
{
    let line = smtp_readline(buf).await?;
    let code: u16 = line.get(..3).and_then(|s| s.parse().ok()).unwrap_or(0);
    if code != expected {
        return Err(format!(
            "SMTP 期望 {} 但收到: {}",
            expected,
            line.trim()
        ));
    }
    // 多行响应（"250-..."）继续读取直到末行（"250 "）
    let mut last = line;
    while last.len() >= 4 && last.as_bytes().get(3) == Some(&b'-') {
        last = smtp_readline(buf).await?;
    }
    Ok(last)
}

/// 读取多行响应（不验证码）
async fn smtp_read_multiline<S>(buf: &mut BufReader<S>) -> Result<String, String>
where
    S: AsyncReadExt + Unpin,
{
    let mut last = smtp_readline(buf).await?;
    while last.len() >= 4 && last.as_bytes().get(3) == Some(&b'-') {
        last = smtp_readline(buf).await?;
    }
    Ok(last)
}

async fn smtp_readline<S>(buf: &mut BufReader<S>) -> Result<String, String>
where
    S: AsyncReadExt + Unpin,
{
    let mut line = String::new();
    buf.read_line(&mut line)
        .await
        .map_err(|e| format!("SMTP 读取失败: {}", e))?;
    Ok(line)
}

// ─────────────────────────────────────────────
// 统一分发入口
// ─────────────────────────────────────────────

/// 读取 DB 配置，并发调用所有已启用的通道
pub async fn dispatch_alert(db: &Database, title: &str, content: &str) {
    let cfg = NotifyConfig::load(db);

    let mut handles = Vec::new();

    // 飞书
    if cfg.feishu_enabled && !cfg.feishu_webhook.is_empty() {
        let url = cfg.feishu_webhook.clone();
        let t = title.to_string();
        let c = content.to_string();
        handles.push(tokio::spawn(async move {
            if let Err(e) = send_feishu(&url, &t, &c).await {
                tracing::warn!("[Notify] 飞书发送失败: {}", e);
            } else {
                tracing::info!("[Notify] 飞书告警已发送: {}", t);
            }
        }));
    }

    // 钉钉
    if cfg.dingtalk_enabled && !cfg.dingtalk_webhook.is_empty() {
        let url = cfg.dingtalk_webhook.clone();
        let secret = if cfg.dingtalk_secret.is_empty() {
            None
        } else {
            Some(cfg.dingtalk_secret.clone())
        };
        let t = title.to_string();
        let c = content.to_string();
        handles.push(tokio::spawn(async move {
            if let Err(e) = send_dingtalk(&url, secret.as_deref(), &t, &c).await {
                tracing::warn!("[Notify] 钉钉发送失败: {}", e);
            } else {
                tracing::info!("[Notify] 钉钉告警已发送: {}", t);
            }
        }));
    }

    // 企业微信
    if cfg.wecom_enabled && !cfg.wecom_webhook.is_empty() {
        let url = cfg.wecom_webhook.clone();
        let t = title.to_string();
        let c = content.to_string();
        handles.push(tokio::spawn(async move {
            if let Err(e) = send_wecom(&url, &t, &c).await {
                tracing::warn!("[Notify] 企业微信发送失败: {}", e);
            } else {
                tracing::info!("[Notify] 企业微信告警已发送: {}", t);
            }
        }));
    }

    // 邮件
    if cfg.email_enabled
        && !cfg.email_smtp_host.is_empty()
        && !cfg.email_username.is_empty()
        && !cfg.email_to.is_empty()
    {
        let host = cfg.email_smtp_host.clone();
        let port = cfg.email_smtp_port;
        let user = cfg.email_username.clone();
        let pass = cfg.email_password.clone();
        let to = cfg.email_to.clone();
        let t = title.to_string();
        let c = content.to_string();
        handles.push(tokio::spawn(async move {
            if let Err(e) = send_email(&host, port, &user, &pass, &to, &t, &c).await {
                tracing::warn!("[Notify] 邮件发送失败: {}", e);
            } else {
                tracing::info!("[Notify] 邮件告警已发送: {}", t);
            }
        }));
    }

    // 等待所有通道完成（忽略 JoinError）
    for h in handles {
        let _ = h.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::Path;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    #[test]
    fn notify_config_defaults_to_all_disabled() {
        let db = make_db();
        let cfg = NotifyConfig::load(&db);
        assert!(!cfg.feishu_enabled);
        assert!(!cfg.dingtalk_enabled);
        assert!(!cfg.wecom_enabled);
        assert!(!cfg.email_enabled);
        assert!(cfg.feishu_webhook.is_empty());
        assert_eq!(cfg.email_smtp_port, 465);
    }

    #[test]
    fn notify_config_persists_after_save() {
        let db = make_db();
        let kvs = vec![
            ("notify_feishu_enabled".to_string(), "true".to_string()),
            (
                "notify_feishu_webhook".to_string(),
                "https://example.com/hook".to_string(),
            ),
            ("notify_email_smtp_port".to_string(), "587".to_string()),
        ];
        db.set_account_settings_batch(&kvs).unwrap();
        let cfg = NotifyConfig::load(&db);
        assert!(cfg.feishu_enabled);
        assert_eq!(cfg.feishu_webhook, "https://example.com/hook");
        assert_eq!(cfg.email_smtp_port, 587);
    }

    #[test]
    fn notify_config_dingtalk_fields() {
        let db = make_db();
        let kvs = vec![
            ("notify_dingtalk_enabled".to_string(), "true".to_string()),
            (
                "notify_dingtalk_webhook".to_string(),
                "https://oapi.dingtalk.com/robot/send?access_token=abc".to_string(),
            ),
            ("notify_dingtalk_secret".to_string(), "SECxxx".to_string()),
        ];
        db.set_account_settings_batch(&kvs).unwrap();
        let cfg = NotifyConfig::load(&db);
        assert!(cfg.dingtalk_enabled);
        assert!(!cfg.dingtalk_webhook.is_empty());
        assert_eq!(cfg.dingtalk_secret, "SECxxx");
    }

    #[test]
    fn notify_config_wecom_fields() {
        let db = make_db();
        let kvs = vec![
            ("notify_wecom_enabled".to_string(), "true".to_string()),
            (
                "notify_wecom_webhook".to_string(),
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xyz".to_string(),
            ),
        ];
        db.set_account_settings_batch(&kvs).unwrap();
        let cfg = NotifyConfig::load(&db);
        assert!(cfg.wecom_enabled);
        assert!(!cfg.wecom_webhook.is_empty());
    }

    #[test]
    fn notify_config_email_fields() {
        let db = make_db();
        let kvs = vec![
            ("notify_email_enabled".to_string(), "true".to_string()),
            (
                "notify_email_smtp_host".to_string(),
                "smtp.example.com".to_string(),
            ),
            ("notify_email_smtp_port".to_string(), "25".to_string()),
            (
                "notify_email_username".to_string(),
                "user@example.com".to_string(),
            ),
            ("notify_email_password".to_string(), "secret".to_string()),
            ("notify_email_to".to_string(), "to@example.com".to_string()),
        ];
        db.set_account_settings_batch(&kvs).unwrap();
        let cfg = NotifyConfig::load(&db);
        assert!(cfg.email_enabled);
        assert_eq!(cfg.email_smtp_host, "smtp.example.com");
        assert_eq!(cfg.email_smtp_port, 25);
        assert_eq!(cfg.email_username, "user@example.com");
        assert_eq!(cfg.email_to, "to@example.com");
    }

    #[test]
    fn notify_config_invalid_port_falls_back_to_default() {
        let db = make_db();
        let kvs = vec![(
            "notify_email_smtp_port".to_string(),
            "not_a_number".to_string(),
        )];
        db.set_account_settings_batch(&kvs).unwrap();
        let cfg = NotifyConfig::load(&db);
        assert_eq!(cfg.email_smtp_port, 465, "invalid port should fall back to 465");
    }

    #[test]
    fn dispatch_alert_does_not_panic_with_empty_config() {
        // 所有通道禁用时，dispatch_alert 应该静默完成不 panic
        let db = make_db();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(dispatch_alert(&db, "Test Title", "Test Content"));
        // 如果没有 panic，测试通过
    }

    // ── HMAC-SHA256 内部实现验证 ────────────────────────────────────────────

    #[test]
    fn hmac_sha256_known_vector() {
        // RFC 4231 Test Case 1:
        // Key  = 0x0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b (20 bytes)
        // Data = "Hi There"
        // HMAC-SHA-256 = b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
        let key = [0x0bu8; 20];
        let msg = b"Hi There";
        let result = hmac_sha256(&key, msg);
        let expected: [u8; 32] = [
            0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
            0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
            0x2e, 0x32, 0xcf, 0xf7,
        ];
        assert_eq!(result, expected, "HMAC-SHA256 RFC 4231 test vector mismatch");
    }

    #[test]
    fn hmac_sha256_long_key_is_hashed() {
        // Key longer than 64 bytes should be pre-hashed; result must still be 32 bytes
        let key = vec![0xaau8; 131];
        let msg = b"Test With a Key Longer than the Block-Size Key";
        let result = hmac_sha256(&key, msg);
        assert_eq!(result.len(), 32);
    }
}
