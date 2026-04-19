use super::*;

impl OauthManager {
    pub async fn poll_oauth_login(login_id: String) -> Result<Option<OAuthResult>, String> {
        let (provider, device_code, code_verifier, uuid, expired, pending) =
            with_sessions(|map| {
                if let Some(s) = map.get(&login_id) {
                    (
                        s.provider.clone(),
                        s.device_code.clone(),
                        s.code_verifier.clone(),
                        s.state_token.clone(),
                        s.expires_at < Instant::now(),
                        s.pending_result.clone(),
                    )
                } else {
                    ("".to_string(), None, None, None, true, None)
                }
            });

        if expired {
            with_sessions(|map| {
                map.remove(&login_id);
            });
            return Err("授权码已过期，请重新发起".to_string());
        }

        if is_localhost_redirect_provider(&provider) {
            if let Some(result) = pending {
                with_sessions(|map| {
                    map.remove(&login_id);
                });
                if let Some(err) = result.error {
                    return Err(err);
                }
                return Ok(Some(result));
            }
            return Ok(None);
        }

        if is_cursor_provider(&provider) {
            return Self::poll_cursor(
                &login_id,
                &uuid.unwrap_or_default(),
                &code_verifier.unwrap_or_default(),
            )
            .await;
        }

        if is_device_flow_provider(&provider) {
            return Self::poll_github(&login_id, &device_code.unwrap_or_default()).await;
        }

        Err(format!("未知 provider: {}", provider))
    }

    async fn poll_cursor(
        login_id: &str,
        uuid: &str,
        code_verifier: &str,
    ) -> Result<Option<OAuthResult>, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CursorPollResp {
            access_token: Option<String>,
            refresh_token: Option<String>,
            auth_id: Option<String>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let url = format!(
            "{}?uuid={}&verifier={}",
            CURSOR_POLL_URL, uuid, code_verifier
        );
        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Cursor 轮询请求失败: {}", e))?;

        let status = resp.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            return Ok(None);
        }

        let body: CursorPollResp = resp
            .json()
            .await
            .map_err(|_| "解析 Cursor 轮询响应失败".to_string())?;

        if let Some(token) = body.access_token.or(body.refresh_token) {
            if !token.is_empty() {
                let email = body
                    .auth_id
                    .as_deref()
                    .filter(|id| id.contains('@'))
                    .map(|id| id.to_string());

                with_sessions(|map| {
                    map.remove(login_id);
                });
                return Ok(Some(OAuthResult {
                    token,
                    access_token: None,
                    refresh_token: None,
                    meta_json: None,
                    email,
                    name: None,
                    provider: "cursor".to_string(),
                    error: None,
                }));
            }
        }
        Ok(None)
    }

    async fn poll_github(
        login_id: &str,
        device_code: &str,
    ) -> Result<Option<OAuthResult>, String> {
        #[derive(Deserialize)]
        struct GhTokenResp {
            access_token: Option<String>,
            error: Option<String>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let resp = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .header("User-Agent", "ai-singularity")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| format!("GitHub token 请求失败: {}", e))?;

        let body: GhTokenResp = resp
            .json()
            .await
            .map_err(|e| format!("解析 GitHub token 响应失败: {}", e))?;

        if let Some(token) = body.access_token {
            if !token.is_empty() {
                let email = Self::fetch_github_user_email(&token).await;
                with_sessions(|map| {
                    map.remove(login_id);
                });
                return Ok(Some(OAuthResult {
                    token,
                    access_token: None,
                    refresh_token: None,
                    meta_json: None,
                    email,
                    name: None,
                    provider: "github_copilot".to_string(),
                    error: None,
                }));
            }
        }

        match body.error.as_deref() {
            Some("authorization_pending") | Some("slow_down") | None => Ok(None),
            Some("expired_token") => {
                with_sessions(|map| {
                    map.remove(login_id);
                });
                Err("GitHub 授权码已过期，请重新发起".to_string())
            }
            Some("access_denied") => {
                with_sessions(|map| {
                    map.remove(login_id);
                });
                Err("用户拒绝了授权".to_string())
            }
            Some(other) => {
                with_sessions(|map| {
                    map.remove(login_id);
                });
                Err(format!("GitHub 授权失败: {}", other))
            }
        }
    }

    async fn fetch_github_user_email(access_token: &str) -> Option<String> {
        #[derive(Deserialize)]
        struct GhUser {
            login: Option<String>,
            email: Option<String>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;

        let resp = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "ai-singularity")
            .send()
            .await
            .ok()?;

        let user: GhUser = resp.json().await.ok()?;
        user.email.or(user.login)
    }

    pub fn cancel_oauth_flow(login_id: Option<String>) -> Result<(), String> {
        with_sessions(|map| match &login_id {
            Some(id) => {
                if let Some(s) = map.remove(id) {
                    let _ = s.cancel_tx.send(true);
                }
            }
            None => {
                for (_, s) in map.drain() {
                    let _ = s.cancel_tx.send(true);
                }
            }
        });
        Ok(())
    }

    pub async fn prepare_oauth_url(
        app: tauri::AppHandle,
    ) -> Result<DeviceFlowStartResponse, String> {
        Self::start_oauth_flow(app, "antigravity".to_string()).await
    }
}
