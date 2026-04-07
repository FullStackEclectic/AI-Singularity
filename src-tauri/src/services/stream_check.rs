use crate::error::AppResult;
use crate::models::{Platform, StreamCheckResult, ProviderConfig};
use crate::db::Database;
use std::time::Instant;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest_eventsource::{EventSource, Event};
use futures::stream::StreamExt;
use reqwest::Client;
use serde_json::json;
use crate::error::AppError;

pub struct StreamCheckService;

impl StreamCheckService {
    pub async fn run_check(db: &Database, provider_id: &str) -> AppResult<StreamCheckResult> {
        // Fetch Provider
        // Fallback for getting the row
        let provider: ProviderConfig = match db.query_row(
            "SELECT id, name, platform, base_url, api_key_id, model_name
             FROM providers WHERE id = ?1", 
             &[&provider_id], 
             |row| {
                let platform_str: String = row.get(2)?;
                let platform = serde_json::from_str(&format!("\"{}\"", platform_str)).unwrap_or(Platform::Custom);
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    platform,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
             }
        ) {
            Ok((id, name, platform, base_url, api_key_id, model_name)) => {
                ProviderConfig {
                    id, name, platform, category: None, base_url, api_key_id, model_name,
                    is_active: false, tool_targets: None, icon: None, icon_color: None, website_url: None, api_key_url: None, notes: None, extra_config: None, sort_order: 0,
                    created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
                }
            },
            Err(_) => {
                return Err(AppError::Other(anyhow::anyhow!("Provider not found or db error")));
            }
        };

        let mut api_key_val = String::new();
        if let Some(key_id) = provider.api_key_id {
            if let Ok(k) = db.query_row("SELECT key_preview FROM api_keys WHERE id = ?1", &[&key_id], |r| r.get::<_, String>(0)) {
                // Actually we need the real key from SecureStore, let's fetch it from keyring instead of key_preview.
                let secret_res = crate::store::SecureStore::get_key(&key_id);
                if let Ok(sec) = secret_res {
                    api_key_val = sec;
                }
            }
        }

        Self::perform_streaming_request(provider.platform, provider.base_url, api_key_val, provider.model_name).await
    }

    async fn perform_streaming_request(
        platform: Platform,
        base_url: Option<String>,
        api_key: String,
        model: String,
    ) -> AppResult<StreamCheckResult> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::Other(anyhow::anyhow!(e.to_string())))?;

        let start_time = Instant::now();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let (url, body) = match platform {
            Platform::Anthropic => {
                headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap_or(HeaderValue::from_static("")));
                headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
                let u = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());
                let endpoint = format!("{}/v1/messages", u.trim_end_matches('/'));
                let b = json!({
                    "model": model,
                    "max_tokens": 5,
                    "messages": [{"role": "user", "content": "hello"}],
                    "stream": true
                });
                (endpoint, b)
            },
            Platform::Gemini => {
                let u = base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
                let endpoint = format!("{}/v1beta/models/{}:streamGenerateContent?key={}", u.trim_end_matches('/'), model, api_key);
                // Gemini SSE
                let b = json!({
                    "contents": [{"role": "user", "parts": [{"text": "hello"}]}]
                });
                (endpoint, b)
            },
            // Fallback for OpenAI & customized
            _ => {
                headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap_or(HeaderValue::from_static("")));
                let mut u = base_url.unwrap_or_else(|| "https://api.openai.com".to_string());
                if !u.ends_with("/v1") && !u.contains("/v1/") {
                    u = format!("{}/v1", u);
                }
                let endpoint = format!("{}/chat/completions", u.trim_end_matches('/'));
                let b = json!({
                    "model": model,
                    "max_tokens": 5,
                    "messages": [{"role": "user", "content": "hello"}],
                    "stream": true
                });
                (endpoint, b)
            }
        };

        let request = client.post(&url).headers(headers).json(&body);
        let mut es = EventSource::new(request).map_err(|e| AppError::Other(anyhow::anyhow!(e.to_string())))?;

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    // connected
                    continue;
                }
                Ok(Event::Message(_)) => {
                    // First token arrived
                    let elapsed = start_time.elapsed().as_millis() as u64;
                    es.close();
                    
                    let status = if elapsed < 1500 {
                        "operational".to_string()
                    } else {
                        "degraded".to_string()
                    };

                    return Ok(StreamCheckResult {
                        status: status,
                        success: true,
                        message: "Success".to_string(),
                        response_time_ms: Some(elapsed),
                        model_used: model,
                    });
                }
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    es.close();
                    break;
                }
                Err(e) => {
                    es.close();
                    return Ok(StreamCheckResult {
                        status: "failed".to_string(),
                        success: false,
                        message: format!("SSE Error: {}", e),
                        response_time_ms: None,
                        model_used: model,
                    });
                }
            }
        }

        // Return failed if no payload received
        Ok(StreamCheckResult {
            status: "failed".to_string(),
            success: false,
            message: "No stream events received".to_string(),
            response_time_ms: None,
            model_used: model,
        })
    }
}
