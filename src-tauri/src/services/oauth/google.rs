use super::config::{
    ANTIGRAVITY_CLIENT_SECRET_ENV, GEMINI_CLIENT_SECRET_ENV, GOOGLE_AUTH_URL, GOOGLE_SCOPES,
    GOOGLE_TOKEN_URL, GOOGLE_USERINFO_URL,
};
use serde::Deserialize;
use std::time::Duration;
use url::Url;

pub fn build_google_auth_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    let mut url = Url::parse(GOOGLE_AUTH_URL).unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("access_type", "offline");
        q.append_pair("scope", GOOGLE_SCOPES);
        q.append_pair("state", state);
        q.append_pair("prompt", "consent");
    }
    url.to_string()
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct GoogleUserInfo {
    pub email: Option<String>,
    pub name: Option<String>,
}

pub async fn exchange_google_code(
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("换取 Google token 失败: {}", e))?;

    let body: GoogleTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 Google token 响应失败: {}", e))?;

    if let Some(err) = body.error {
        return Err(format!(
            "Google 授权失败: {} ({})",
            err,
            body.error_description.unwrap_or_default()
        ));
    }

    let access_token = body
        .access_token
        .ok_or("Google token 响应缺少 access_token")?;
    Ok((access_token, body.refresh_token))
}

pub async fn fetch_google_userinfo(access_token: &str) -> Option<GoogleUserInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .ok()?;
    resp.json::<GoogleUserInfo>().await.ok()
}

pub fn get_google_client_secret(provider: &str) -> Result<String, String> {
    let (env_name, label) = match provider {
        "gemini" => (GEMINI_CLIENT_SECRET_ENV, "Gemini"),
        "antigravity" => (ANTIGRAVITY_CLIENT_SECRET_ENV, "Antigravity"),
        other => return Err(format!("{} 不是 Google OAuth provider", other)),
    };

    std::env::var(env_name).map_err(|_| {
        format!(
            "{} OAuth 尚未配置 client secret。请设置环境变量 {} 后重试。",
            label, env_name
        )
    })
}
