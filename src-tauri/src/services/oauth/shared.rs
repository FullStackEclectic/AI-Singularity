use super::config::{GOOGLE_OAUTH_CALLBACK_PATH, OAUTH_TIMEOUT_SECS};
use base64::Engine as _;
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};
use url::Url;

static CALLBACK_SUCCESS_HTML: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<html><body style='font-family:sans-serif;background:#0f172a;color:#e2e8f0;\
padding:32px;text-align:center;'>\
<h2 style='color:#22c55e;'>✅ 授权成功</h2>\
<p>可以关闭此窗口并返回 AI Singularity。</p>\
<script>setTimeout(function(){window.close();},1500);</script>\
</body></html>";

static CALLBACK_FAIL_HTML: &str =
    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<html><body style='font-family:sans-serif;background:#0f172a;color:#e2e8f0;\
padding:32px;text-align:center;'>\
<h2 style='color:#ef4444;'>❌ 授权失败</h2>\
<p>state 校验失败或回调参数缺失，请重新尝试。</p>\
</body></html>";

pub fn generate_token(len: usize) -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..len).map(|_| rand::thread_rng().gen::<u8>()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

pub fn sha256_b64(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize().as_slice())
}

pub async fn exchange_pkce_code(
    token_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];
    if !client_id.is_empty() {
        params.push(("client_id", client_id));
    }

    let resp = client
        .post(token_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("PKCE token exchange 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("PKCE token exchange 失败: HTTP {} — {}", status, body));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析 PKCE token 响应失败: {}", e))
}

pub fn decode_jwt_claim(token: &str, claim: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let padding = (4 - parts[1].len() % 4) % 4;
    let padded = format!("{}{}", parts[1], "=".repeat(padding));
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&padded)
        .ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    payload
        .get(claim)?
        .as_str()
        .filter(|value| !value.is_empty())
        .map(String::from)
}

pub fn decode_any_jwt_claim(token: &str, claims: &[&str]) -> Option<String> {
    for claim in claims {
        if let Some(value) = decode_jwt_claim(token, claim) {
            return Some(value);
        }
    }
    None
}

pub async fn wait_for_callback(
    port: u16,
    expected_state: String,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<(String, String), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定回调端口 {} 失败: {}", port, e))?;

    let redirect_uri = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (mut stream, _) = res.map_err(|e| format!("接受连接失败: {}", e))?;

                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                let params = request
                    .lines()
                    .next()
                    .and_then(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        parts.get(1).copied()
                    })
                    .and_then(|path| {
                        Url::parse(&format!("http://127.0.0.1:{}{}", port, path)).ok()
                    })
                    .map(|url| {
                        url.query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect::<HashMap<String, String>>()
                    })
                    .unwrap_or_default();

                let received_state = params.get("state").cloned().unwrap_or_default();
                let code = params.get("code").cloned().unwrap_or_default();

                if received_state != expected_state || code.is_empty() {
                    let _ = stream.write_all(CALLBACK_FAIL_HTML.as_bytes()).await;
                    continue;
                }

                let _ = stream.write_all(CALLBACK_SUCCESS_HTML.as_bytes()).await;
                return Ok((code, redirect_uri));
            }
            _ = cancel_rx.changed() => {
                return Err("授权已取消".to_string());
            }
            _ = tokio::time::sleep(Duration::from_secs(OAUTH_TIMEOUT_SECS)) => {
                return Err("等待授权超时，请重试".to_string());
            }
        }
    }
}

pub(crate) fn callback_success_html() -> &'static str {
    CALLBACK_SUCCESS_HTML
}

pub(crate) fn callback_fail_html() -> &'static str {
    CALLBACK_FAIL_HTML
}
