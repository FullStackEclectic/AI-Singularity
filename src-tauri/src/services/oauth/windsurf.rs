use super::config::{
    OAUTH_TIMEOUT_SECS, WINDSURF_AUTH_BASE_URL, WINDSURF_CLIENT_ID, WINDSURF_REGISTER_API_URL,
};
use super::shared::{callback_fail_html, callback_success_html};
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};
use url::Url;

pub fn build_windsurf_auth_url(redirect_uri: &str, state: &str) -> String {
    let mut url = Url::parse(&format!("{}/windsurf/signin", WINDSURF_AUTH_BASE_URL)).unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "token");
        q.append_pair("client_id", WINDSURF_CLIENT_ID);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("state", state);
        q.append_pair("prompt", "login");
        q.append_pair("redirect_parameters_type", "query");
        q.append_pair("workflow", "onboarding");
    }
    url.to_string()
}

pub async fn wait_for_windsurf_callback(
    port: u16,
    expected_state: String,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定回调端口 {} 失败: {}", port, e))?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (mut stream, _) = res.map_err(|e| format!("接受连接失败: {}", e))?;

                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                let params = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|path| Url::parse(&format!("http://127.0.0.1:{}{}", port, path)).ok())
                    .map(|url| {
                        url.query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect::<HashMap<String, String>>()
                    })
                    .unwrap_or_default();

                let received_state = params.get("state").cloned().unwrap_or_default();
                let access_token = params.get("access_token").cloned().unwrap_or_default();
                let error = params.get("error").cloned();

                if received_state != expected_state {
                    let _ = stream.write_all(callback_fail_html().as_bytes()).await;
                    continue;
                }

                if let Some(err) = error {
                    let _ = stream.write_all(callback_fail_html().as_bytes()).await;
                    return Err(format!("Windsurf 授权拒绝: {}", err));
                }

                if access_token.is_empty() {
                    let _ = stream.write_all(callback_fail_html().as_bytes()).await;
                    continue;
                }

                let _ = stream.write_all(callback_success_html().as_bytes()).await;
                return Ok(access_token);
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

pub async fn windsurf_register_user(
    access_token: &str,
) -> Result<(String, Option<String>, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let body = serde_json::json!({ "firebase_id_token": access_token });
    let url = format!(
        "{}/exa.seat_management_pb.SeatManagementService/RegisterUser",
        WINDSURF_REGISTER_API_URL
    );

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "ai-singularity")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Windsurf RegisterUser 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Windsurf RegisterUser 失败: HTTP {} — {}",
            status, text
        ));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Windsurf RegisterUser 响应失败: {}", e))?;

    let api_key = value
        .get("apiKey")
        .or_else(|| value.get("api_key"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or("Windsurf RegisterUser 响应缺少 apiKey")?;

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    Ok((api_key, name, None))
}
