use super::config::{TRAE_AUTH_CLIENT_ID, TRAE_LOGIN_GUIDANCE_URL};
use std::time::Duration;

pub async fn trae_get_login_url(redirect_uri: &str, state: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let body = serde_json::json!({
        "client_id": TRAE_AUTH_CLIENT_ID,
        "redirect_uri": redirect_uri,
        "state": state
    });

    let resp = client
        .post(TRAE_LOGIN_GUIDANCE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Trae GetLoginGuidance 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Trae GetLoginGuidance 失败: HTTP {} — {}",
            status, text
        ));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Trae GetLoginGuidance 响应失败: {}", e))?;

    value
        .get("data")
        .and_then(|data| {
            data.get("authUrl")
                .or_else(|| data.get("auth_url"))
                .or_else(|| data.get("url"))
        })
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(String::from)
        .or_else(|| {
            Some(format!(
                "https://www.trae.ai/oauth/authorization?client_id={}&redirect_uri={}&state={}",
                TRAE_AUTH_CLIENT_ID,
                urlencoding::encode(redirect_uri),
                state
            ))
        })
        .ok_or("无法获取 Trae 登录 URL".to_string())
}

pub async fn trae_get_user_info(
    token: &str,
    login_host: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

    let base = if login_host.starts_with("http") {
        login_host.trim_end_matches('/').to_string()
    } else {
        "https://api.marscode.com".to_string()
    };
    let url = format!("{}{}", base, "/cloudide/api/v3/trae/GetUserInfo");

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("Trae GetUserInfo 请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Ok((None, None));
    }

    let value: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    let data = value.get("data").unwrap_or(&serde_json::Value::Null);
    let email = data
        .get("email")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let name = data
        .get("name")
        .or_else(|| data.get("nickname"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    Ok((email, name))
}
