use crate::db::Database;
use crate::error::AppResult;
use crate::models::{Platform, ProviderConfig};
use crate::services::provider::ProviderService;
use crate::services::event_bus::EventBus;
use crate::store::SecureStore;
use anyhow::anyhow;
use tauri::{AppHandle, State};

#[derive(serde::Deserialize)]
pub struct FetchProviderModelsRequest {
    platform: Platform,
    base_url: Option<String>,
    api_key_value: Option<String>,
    api_key_id: Option<String>,
}

#[tauri::command]
pub fn get_providers(db: State<'_, Database>) -> AppResult<Vec<ProviderConfig>> {
    ProviderService::new(&*db).list_providers()
}

#[tauri::command]
pub fn add_provider(
    provider: ProviderConfig,
    db: State<'_, Database>,
    app: AppHandle,
) -> AppResult<()> {
    ProviderService::new(&*db).add_provider(provider)?;
    crate::tray::update_tray_menu(&app);
    EventBus::emit_data_changed(&app, "providers", "add", "provider.add");
    Ok(())
}

#[tauri::command]
pub fn update_provider(
    provider: ProviderConfig,
    db: State<'_, Database>,
    app: AppHandle,
) -> AppResult<()> {
    ProviderService::new(&*db).update_provider(provider)?;
    crate::tray::update_tray_menu(&app);
    EventBus::emit_data_changed(&app, "providers", "update", "provider.update");
    Ok(())
}

/// 切换激活 Provider（不再需要 ai_tool，后端基于 id 全局互斥）
#[tauri::command]
pub fn switch_provider(id: String, db: State<'_, Database>, app: AppHandle) -> AppResult<()> {
    ProviderService::new(&*db).switch_provider(&id)?;
    crate::tray::update_tray_menu(&app);
    EventBus::emit_data_changed(&app, "providers", "switch", "provider.switch");
    Ok(())
}

#[tauri::command]
pub fn delete_provider(id: String, db: State<'_, Database>, app: AppHandle) -> AppResult<()> {
    ProviderService::new(&*db).delete_provider(&id)?;
    crate::tray::update_tray_menu(&app);
    EventBus::emit_data_changed(&app, "providers", "delete", "provider.delete");
    Ok(())
}

#[tauri::command]
pub fn update_providers_order(
    ids: Vec<String>,
    db: State<'_, Database>,
    app: AppHandle,
) -> AppResult<()> {
    ProviderService::new(&*db).reorder_providers(ids)?;
    crate::tray::update_tray_menu(&app);
    EventBus::emit_data_changed(&app, "providers", "reorder", "provider.reorder");
    Ok(())
}

fn normalize_model_name(raw: &str) -> String {
    raw.strip_prefix("models/").unwrap_or(raw).to_string()
}

fn openai_like_base(platform: &Platform) -> Option<&'static str> {
    match platform {
        Platform::OpenAI => Some("https://api.openai.com"),
        Platform::Custom => Some("https://api.openai.com"),
        Platform::DeepSeek => Some("https://api.deepseek.com"),
        Platform::Aliyun => Some("https://dashscope.aliyuncs.com/compatible-mode"),
        Platform::Bytedance => Some("https://ark.cn-beijing.volces.com/api/v3"),
        Platform::Moonshot => Some("https://api.moonshot.cn"),
        Platform::Zhipu => Some("https://open.bigmodel.cn/api/paas/v4"),
        Platform::MiniMax => Some("https://api.minimax.chat/v1"),
        Platform::StepFun => Some("https://api.stepfun.com"),
        Platform::AwsBedrock => None,
        Platform::NvidiaNim => Some("https://integrate.api.nvidia.com"),
        Platform::AzureOpenAI => None,
        Platform::SiliconFlow => Some("https://api.siliconflow.cn"),
        Platform::OpenRouter => Some("https://openrouter.ai/api"),
        Platform::Groq => Some("https://api.groq.com/openai"),
        Platform::Mistral => Some("https://api.mistral.ai"),
        Platform::XAi => Some("https://api.x.ai"),
        Platform::Cohere => Some("https://api.cohere.com/compatibility"),
        Platform::Perplexity => Some("https://api.perplexity.ai"),
        Platform::TogetherAi => Some("https://api.together.xyz"),
        Platform::Ollama => Some("http://127.0.0.1:11434"),
        Platform::HuggingFace => None,
        Platform::Replicate => None,
        Platform::Copilot | Platform::Auth0IDE => None,
        Platform::Anthropic | Platform::Gemini => None,
    }
}

#[tauri::command]
pub async fn fetch_provider_models(request: FetchProviderModelsRequest) -> AppResult<Vec<String>> {
    let secret = if let Some(secret) = request.api_key_value.filter(|v| !v.trim().is_empty()) {
        Some(secret)
    } else if let Some(key_id) = request.api_key_id.filter(|v| !v.trim().is_empty()) {
        Some(SecureStore::get_key(&key_id)?)
    } else {
        None
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let mut models: Vec<String> = match request.platform {
        Platform::Anthropic => {
            let api_key = secret.ok_or_else(|| anyhow!("Anthropic 拉取模型需要 API Key"))?;
            let base = request
                .base_url
                .unwrap_or_else(|| "https://api.anthropic.com".to_string());
            let url = format!("{}/v1/models", base.trim_end_matches('/'));
            let value: serde_json::Value = client
                .get(url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            value["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["id"].as_str().or_else(|| item["name"].as_str()))
                .map(normalize_model_name)
                .collect()
        }
        Platform::Gemini => {
            let api_key = secret.ok_or_else(|| anyhow!("Gemini 拉取模型需要 API Key"))?;
            let base = request
                .base_url
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
            let url = format!(
                "{}/v1beta/models?key={}",
                base.trim_end_matches('/'),
                api_key
            );
            let value: serde_json::Value = client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            value["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["name"].as_str())
                .map(normalize_model_name)
                .collect()
        }
        other => {
            let api_key = secret.ok_or_else(|| anyhow!("拉取模型需要 API Key"))?;
            let base = request
                .base_url
                .or_else(|| openai_like_base(&other).map(|s| s.to_string()))
                .ok_or_else(|| {
                    anyhow!("当前平台暂不支持自动推断模型列表地址，请先填写 Base URL")
                })?;
            let url = format!("{}/v1/models", base.trim_end_matches('/'));
            let value: serde_json::Value = client
                .get(url)
                .bearer_auth(api_key)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            value["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["id"].as_str().or_else(|| item["name"].as_str()))
                .map(normalize_model_name)
                .collect()
        }
    };

    models.sort();
    models.dedup();

    if models.is_empty() {
        return Err(anyhow!("没有从供应商返回任何模型").into());
    }

    Ok(models)
}
