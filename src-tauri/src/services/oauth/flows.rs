use super::*;

impl OauthManager {
    pub(super) async fn start_localhost_redirect_flow(
        app: tauri::AppHandle,
        provider: &str,
    ) -> Result<DeviceFlowStartResponse, String> {
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("绑定本地端口失败: {}", e))?;
        let port = probe
            .local_addr()
            .map_err(|e| format!("获取本地端口失败: {}", e))?
            .port();
        drop(probe);

        let state_token = generate_token(24);
        let login_id = generate_token(16);

        let (auth_url, redirect_uri, extra_state) = match provider {
            "antigravity" => {
                let redir = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);
                let url = build_google_auth_url(ANTIGRAVITY_CLIENT_ID, &redir, &state_token);
                (url, redir, None::<String>)
            }
            "gemini" => {
                let redir = format!("http://127.0.0.1:{}{}", port, GOOGLE_OAUTH_CALLBACK_PATH);
                let url = build_google_auth_url(GEMINI_CLIENT_ID, &redir, &state_token);
                (url, redir, None)
            }
            "windsurf" => {
                let redir = format!("http://127.0.0.1:{}{}", port, WINDSURF_CALLBACK_PATH);
                let url = build_windsurf_auth_url(&redir, &state_token);
                (url, redir, None)
            }
            "codex" => {
                let redir = format!(
                    "http://localhost:{}{}",
                    CODEX_CALLBACK_PORT, CODEX_CALLBACK_PATH
                );
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let url = format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&originator=codex_vscode&codex_cli_simplified_flow=true",
                    CODEX_AUTH_URL,
                    CODEX_CLIENT_ID,
                    urlencoding::encode(&redir),
                    urlencoding::encode(CODEX_SCOPES),
                    code_challenge,
                    state_token
                );
                (url, redir, Some(code_verifier))
            }
            "kiro" => {
                let redir = format!("http://127.0.0.1:{}{}", port, KIRO_CALLBACK_PATH);
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let url = format!(
                    "{}?state={}&code_challenge={}&code_challenge_method=S256&redirect_uri={}&redirect_from=KiroIDE",
                    KIRO_AUTH_URL,
                    urlencoding::encode(&state_token),
                    urlencoding::encode(&code_challenge),
                    urlencoding::encode(&redir)
                );
                (url, redir, Some(code_verifier))
            }
            "trae" => {
                let redir = format!("http://127.0.0.1:{}{}", port, TRAE_CALLBACK_PATH);
                let url = format!(
                    "https://www.trae.ai/oauth/authorize?client_id={}&redirect_uri={}&state={}",
                    TRAE_AUTH_CLIENT_ID,
                    urlencoding::encode(&redir),
                    state_token
                );
                (url, redir, None)
            }
            "zed" => {
                let redir = format!("http://127.0.0.1:{}{}", port, ZED_CALLBACK_PATH);
                let url = format!(
                    "{}?app_callback_url={}&state={}",
                    ZED_SIGNIN_URL,
                    urlencoding::encode(&redir),
                    state_token
                );
                (url, redir, None)
            }
            _ => {
                return Err(format!(
                    "「{}」OAuth 暂未支持，敬请期待后续更新。目前可使用「导入」Tab 导入账号。",
                    provider
                ));
            }
        };

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(OAUTH_TIMEOUT_SECS);
        let provider_owned = provider.to_string();
        let code_verifier_opt = extra_state.clone();

        with_sessions(|map| {
            map.insert(
                login_id.clone(),
                OAuthSession {
                    provider: provider_owned.clone(),
                    callback_port: Some(port),
                    pending_result: None,
                    state_token: Some(state_token.clone()),
                    code_verifier: extra_state,
                    device_code: None,
                    cancel_tx,
                    expires_at,
                },
            );
        });

        let login_id_bg = login_id.clone();
        let app_handle = app.clone();
        let state_bg = state_token.clone();
        let redirect_uri_bg = redirect_uri.clone();
        tokio::spawn(async move {
            let oauth_result = match provider_owned.as_str() {
                "windsurf" => match wait_for_windsurf_callback(port, state_bg, cancel_rx).await {
                    Ok(access_token) => match windsurf_register_user(&access_token).await {
                        Ok((api_key, name, email)) => Ok(OAuthResult {
                            token: api_key,
                            access_token: None,
                            refresh_token: None,
                            meta_json: None,
                            email,
                            name,
                            provider: "windsurf".to_string(),
                            error: None,
                        }),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                },
                "antigravity" | "gemini" => match get_google_client_secret(provider_owned.as_str())
                {
                    Ok(client_secret_own) => {
                        let client_id_owned = match provider_owned.as_str() {
                            "gemini" => GEMINI_CLIENT_ID.to_string(),
                            _ => ANTIGRAVITY_CLIENT_ID.to_string(),
                        };
                        match wait_for_callback(port, state_bg, cancel_rx).await {
                            Ok((code, redir)) => match exchange_google_code(
                                &code,
                                &redir,
                                &client_id_owned,
                                &client_secret_own,
                            )
                            .await
                            {
                                Ok((access_token, refresh_token)) => {
                                    let user_info = fetch_google_userinfo(&access_token).await;
                                    let refresh_token_to_store = refresh_token.clone();
                                    let token = refresh_token.unwrap_or(access_token.clone());
                                    Ok(OAuthResult {
                                        token,
                                        access_token: Some(access_token),
                                        refresh_token: refresh_token_to_store,
                                        meta_json: None,
                                        email: user_info.as_ref().and_then(|u| u.email.clone()),
                                        name: user_info.as_ref().and_then(|u| u.name.clone()),
                                        provider: provider_owned.clone(),
                                        error: None,
                                    })
                                }
                                Err(e) => Err(e),
                            },
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                },
                "codex" => {
                    let cb_port = CODEX_CALLBACK_PORT;
                    match wait_for_callback(cb_port, state_bg, cancel_rx).await {
                        Ok((code, _redir)) => {
                            let verifier = code_verifier_opt.clone().unwrap_or_default();
                            let redir = format!(
                                "http://localhost:{}{}",
                                CODEX_CALLBACK_PORT, CODEX_CALLBACK_PATH
                            );
                            match exchange_pkce_code(
                                CODEX_TOKEN_URL,
                                CODEX_CLIENT_ID,
                                &code,
                                &redir,
                                &verifier,
                            )
                            .await
                            {
                                Ok(token_json) => {
                                    let access_token = token_json["access_token"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    let refresh_token = token_json["refresh_token"]
                                        .as_str()
                                        .filter(|s| !s.is_empty())
                                        .map(String::from);
                                    let id_token = token_json["id_token"]
                                        .as_str()
                                        .filter(|s| !s.is_empty())
                                        .map(String::from);
                                    let email = decode_jwt_claim(&access_token, "email");
                                    let name = decode_jwt_claim(&access_token, "name");
                                    let account_id = decode_any_jwt_claim(
                                        &access_token,
                                        &["chatgpt_account_id", "account_id", "workspace_id"],
                                    )
                                    .or_else(|| {
                                        id_token.as_deref().and_then(|token| {
                                            decode_any_jwt_claim(
                                                token,
                                                &[
                                                    "chatgpt_account_id",
                                                    "account_id",
                                                    "workspace_id",
                                                ],
                                            )
                                        })
                                    });
                                    let meta_json = serde_json::json!({
                                        "auth_mode": "oauth",
                                        "id_token": id_token,
                                        "account_id": account_id,
                                        "last_refresh": chrono::Utc::now().to_rfc3339(),
                                    })
                                    .to_string();
                                    Ok(OAuthResult {
                                        token: access_token.clone(),
                                        access_token: Some(access_token),
                                        refresh_token,
                                        meta_json: Some(meta_json),
                                        email,
                                        name,
                                        provider: "codex".to_string(),
                                        error: None,
                                    })
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                "kiro" => match wait_for_callback(port, state_bg, cancel_rx).await {
                    Ok((code, redir)) => {
                        let verifier = code_verifier_opt.clone().unwrap_or_default();
                        match exchange_pkce_code(KIRO_TOKEN_URL, "", &code, &redir, &verifier).await
                        {
                            Ok(token_json) => {
                                let access_token = token_json["accessToken"]
                                    .as_str()
                                    .or_else(|| token_json["access_token"].as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let email = token_json["email"]
                                    .as_str()
                                    .map(String::from)
                                    .or_else(|| decode_jwt_claim(&access_token, "email"));
                                let name = token_json["name"]
                                    .as_str()
                                    .map(String::from)
                                    .or_else(|| decode_jwt_claim(&access_token, "name"));
                                Ok(OAuthResult {
                                    token: access_token,
                                    access_token: None,
                                    refresh_token: None,
                                    meta_json: None,
                                    email,
                                    name,
                                    provider: "kiro".to_string(),
                                    error: None,
                                })
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                },
                "trae" => match trae_get_login_url(&redirect_uri_bg, &state_bg).await {
                    Ok(real_auth_url) => {
                        use tauri_plugin_opener::OpenerExt;
                        let _ = app_handle.opener().open_url(&real_auth_url, None::<String>);
                        match wait_for_callback(port, state_bg.clone(), cancel_rx).await {
                            Ok((refresh_token, _redir)) => {
                                let (email, name) = trae_get_user_info(
                                    &refresh_token,
                                    &redirect_uri_bg,
                                )
                                .await
                                .unwrap_or((None, None));
                                Ok(OAuthResult {
                                    token: refresh_token,
                                    access_token: None,
                                    refresh_token: None,
                                    meta_json: None,
                                    email,
                                    name,
                                    provider: "trae".to_string(),
                                    error: None,
                                })
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                },
                "zed" => match wait_for_callback(port, state_bg, cancel_rx).await {
                    Ok((access_token, _)) => {
                        let email = decode_jwt_claim(&access_token, "email");
                        let name = decode_jwt_claim(&access_token, "name");
                        Ok(OAuthResult {
                            token: access_token,
                            access_token: None,
                            refresh_token: None,
                            meta_json: None,
                            email,
                            name,
                            provider: "zed".to_string(),
                            error: None,
                        })
                    }
                    Err(e) => Err(e),
                },
                _ => Err(format!(
                    "OAuth provider {} 尚未接入后台回调处理",
                    provider_owned
                )),
            };

            match oauth_result {
                Ok(result) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(result);
                        }
                    });
                    use tauri::Manager;
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    use tauri::Emitter;
                    let _ = app_handle.emit("oauth-callback-received", &login_id_bg);
                }
                Err(e) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(OAuthResult {
                                token: String::new(),
                                access_token: None,
                                refresh_token: None,
                                meta_json: None,
                                email: None,
                                name: None,
                                provider: String::new(),
                                error: Some(e),
                            });
                        }
                    });
                }
            }
        });

        {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url(&auth_url, None::<String>);
        }

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code: String::new(),
            verification_uri: auth_url,
            expires_in: OAUTH_TIMEOUT_SECS,
            interval_seconds: 2,
        })
    }

    pub(super) async fn start_server_poll_flow(
        app: tauri::AppHandle,
        provider: &str,
    ) -> Result<DeviceFlowStartResponse, String> {
        let (verification_uri, poll_state, login_id) = match provider {
            "qoder" => {
                let code_verifier = generate_token(32);
                let code_challenge = sha256_b64(&code_verifier);
                let nonce = generate_token(16);
                let login_id = generate_token(16);

                let auth_url = format!(
                    "{}?nonce={}&challenge={}&challenge_method=S256&client_id={}",
                    QODER_LOGIN_URL,
                    urlencoding::encode(&nonce),
                    urlencoding::encode(&code_challenge),
                    urlencoding::encode(QODER_CLIENT_ID)
                );

                let state_json = serde_json::json!({
                    "nonce": nonce,
                    "verifier": code_verifier,
                    "challenge_method": "S256"
                })
                .to_string();

                (auth_url, state_json, login_id)
            }
            "codebuddy" => {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .build()
                    .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;
                let url = format!(
                    "{}{}/auth/state?platform=ide",
                    CODEBUDDY_API_URL, CODEBUDDY_API_PREFIX
                );
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({}))
                    .send()
                    .await
                    .map_err(|e| format!("CodeBuddy auth/state 请求失败: {}", e))?;

                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("解析 CodeBuddy auth/state 响应失败: {}", e))?;

                let data = body.get("data").cloned().unwrap_or(serde_json::Value::Null);
                let state = data
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let auth_url = data
                    .get("authUrl")
                    .or_else(|| data.get("url"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| format!("{}/login?state={}", CODEBUDDY_API_URL, state));

                let login_id = generate_token(16);
                let state_json = serde_json::json!({ "state": state }).to_string();

                (auth_url, state_json, login_id)
            }
            _ => return Err(format!("未知的服务端轮询渠道: {}", provider)),
        };

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(OAUTH_TIMEOUT_SECS);
        let provider_owned = provider.to_string();
        let poll_state_clone = poll_state.clone();

        with_sessions(|map| {
            map.insert(
                login_id.clone(),
                OAuthSession {
                    provider: provider_owned.clone(),
                    callback_port: None,
                    pending_result: None,
                    state_token: None,
                    code_verifier: None,
                    device_code: Some(poll_state.clone()),
                    cancel_tx,
                    expires_at,
                },
            );
        });

        let login_id_bg = login_id.clone();
        let app_bg = app.clone();
        tokio::spawn(async move {
            let result = match provider_owned.as_str() {
                "qoder" => {
                    let state: serde_json::Value =
                        serde_json::from_str(&poll_state_clone).unwrap_or(serde_json::Value::Null);
                    let nonce = state["nonce"].as_str().unwrap_or("").to_string();
                    let verifier = state["verifier"].as_str().unwrap_or("").to_string();
                    let method = state["challenge_method"]
                        .as_str()
                        .unwrap_or("S256")
                        .to_string();

                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(15))
                        .build()
                        .ok();
                    let mut found: Option<OAuthResult> = None;
                    let deadline = tokio::time::Instant::now()
                        + tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS);

                    loop {
                        if tokio::time::Instant::now() > deadline {
                            break;
                        }
                        if *cancel_rx.borrow() {
                            break;
                        }

                        if let Some(ref c) = client {
                            let url = format!(
                                "{}{}?nonce={}&verifier={}&challenge_method={}",
                                QODER_OPENAPI_URL,
                                QODER_POLL_PATH,
                                urlencoding::encode(&nonce),
                                urlencoding::encode(&verifier),
                                urlencoding::encode(&method)
                            );
                            if let Ok(resp) = c.get(&url).send().await {
                                if resp.status().is_success() {
                                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                                        let token = body
                                            .get("token")
                                            .and_then(|v| v.as_str())
                                            .filter(|s| !s.is_empty())
                                            .map(String::from);
                                        if let Some(tk) = token {
                                            let (email, name) = if let Ok(ui_resp) = c
                                                .get(&format!(
                                                    "{}{}",
                                                    QODER_OPENAPI_URL, QODER_USERINFO_PATH
                                                ))
                                                .bearer_auth(&tk)
                                                .send()
                                                .await
                                            {
                                                if let Ok(ui) =
                                                    ui_resp.json::<serde_json::Value>().await
                                                {
                                                    let email = ui
                                                        .get("email")
                                                        .and_then(|v| v.as_str())
                                                        .map(String::from);
                                                    let name = ui
                                                        .get("name")
                                                        .and_then(|v| v.as_str())
                                                        .map(String::from);
                                                    (email, name)
                                                } else {
                                                    (None, None)
                                                }
                                            } else {
                                                (None, None)
                                            };
                                            found = Some(OAuthResult {
                                                token: tk,
                                                access_token: None,
                                                refresh_token: None,
                                                meta_json: None,
                                                email,
                                                name,
                                                provider: "qoder".to_string(),
                                                error: None,
                                            });
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    found.ok_or("Qoder 授权超时或已取消".to_string())
                }
                "codebuddy" => {
                    let state: serde_json::Value =
                        serde_json::from_str(&poll_state_clone).unwrap_or(serde_json::Value::Null);
                    let cb_state = state["state"].as_str().unwrap_or("").to_string();

                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(15))
                        .build()
                        .ok();
                    let mut found: Option<OAuthResult> = None;
                    let deadline = tokio::time::Instant::now()
                        + tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS);

                    loop {
                        if tokio::time::Instant::now() > deadline {
                            break;
                        }
                        if *cancel_rx.borrow() {
                            break;
                        }

                        if let Some(ref c) = client {
                            let url = format!(
                                "{}{}/auth/token?state={}",
                                CODEBUDDY_API_URL, CODEBUDDY_API_PREFIX, cb_state
                            );
                            if let Ok(resp) = c.get(&url).send().await {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    let code =
                                        body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
                                    if code == 0 || code == 200 {
                                        if let Some(data) = body.get("data") {
                                            let access_token = data
                                                .get("accessToken")
                                                .or_else(|| data.get("access_token"))
                                                .and_then(|v| v.as_str())
                                                .filter(|s| !s.is_empty())
                                                .map(String::from);
                                            if let Some(tk) = access_token {
                                                let email = data
                                                    .get("email")
                                                    .and_then(|v| v.as_str())
                                                    .map(String::from);
                                                let name = data
                                                    .get("nickname")
                                                    .or_else(|| data.get("name"))
                                                    .and_then(|v| v.as_str())
                                                    .map(String::from);
                                                found = Some(OAuthResult {
                                                    token: tk,
                                                    access_token: None,
                                                    refresh_token: None,
                                                    meta_json: None,
                                                    email,
                                                    name,
                                                    provider: "codebuddy".to_string(),
                                                    error: None,
                                                });
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    found.ok_or("CodeBuddy 授权超时或已取消".to_string())
                }
                _ => Err("未知渠道".to_string()),
            };

            match result {
                Ok(r) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(r);
                        }
                    });
                    use tauri::Manager;
                    if let Some(window) = app_bg.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    use tauri::Emitter;
                    let _ = app_bg.emit("oauth-callback-received", &login_id_bg);
                }
                Err(e) => {
                    with_sessions(|map| {
                        if let Some(s) = map.get_mut(&login_id_bg) {
                            s.pending_result = Some(OAuthResult {
                                token: String::new(),
                                access_token: None,
                                refresh_token: None,
                                meta_json: None,
                                email: None,
                                name: None,
                                provider: String::new(),
                                error: Some(e),
                            });
                        }
                    });
                }
            }
        });

        {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url(&verification_uri, None::<String>);
        }

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code: String::new(),
            verification_uri,
            expires_in: OAUTH_TIMEOUT_SECS,
            interval_seconds: 2,
        })
    }

    pub(super) async fn start_github_device_flow() -> Result<DeviceFlowStartResponse, String> {
        #[derive(Deserialize)]
        struct GhDeviceCodeResp {
            device_code: String,
            user_code: String,
            verification_uri: String,
            expires_in: u64,
            interval: Option<u64>,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let resp = client
            .post(GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .header("User-Agent", "ai-singularity")
            .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", GITHUB_SCOPE)])
            .send()
            .await
            .map_err(|e| format!("请求 GitHub 设备码失败（请检查网络）: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub 设备码请求失败: HTTP {} — {}", status, body));
        }

        let payload: GhDeviceCodeResp = resp
            .json()
            .await
            .map_err(|e| format!("解析 GitHub 设备码响应失败: {}", e))?;

        let login_id = generate_token(16);
        let interval = payload.interval.unwrap_or(5).max(5);
        let (cancel_tx, _) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(payload.expires_in);

        with_sessions(|map| {
            map.insert(
                login_id.clone(),
                OAuthSession {
                    provider: "github_copilot".to_string(),
                    callback_port: None,
                    pending_result: None,
                    state_token: None,
                    code_verifier: None,
                    device_code: Some(payload.device_code.clone()),
                    cancel_tx,
                    expires_at,
                },
            );
        });

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code: payload.user_code,
            verification_uri: payload.verification_uri,
            expires_in: payload.expires_in,
            interval_seconds: interval,
        })
    }

    pub(super) fn start_cursor_flow() -> Result<DeviceFlowStartResponse, String> {
        let code_verifier = generate_token(32);
        let code_challenge = sha256_b64(&code_verifier);
        let uuid = uuid::Uuid::new_v4().to_string();
        let login_id = generate_token(16);

        let verification_uri = format!(
            "{}?challenge={}&uuid={}&mode=login",
            CURSOR_LOGIN_URL, code_challenge, uuid
        );

        let (cancel_tx, _) = watch::channel(false);
        let expires_at = Instant::now() + Duration::from_secs(300);

        with_sessions(|map| {
            map.insert(
                login_id.clone(),
                OAuthSession {
                    provider: "cursor".to_string(),
                    callback_port: None,
                    pending_result: None,
                    state_token: Some(uuid.clone()),
                    code_verifier: Some(code_verifier.clone()),
                    device_code: None,
                    cancel_tx,
                    expires_at,
                },
            );
        });

        Ok(DeviceFlowStartResponse {
            login_id,
            user_code: String::new(),
            verification_uri,
            expires_in: 300,
            interval_seconds: 2,
        })
    }
}
