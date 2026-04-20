use crate::models::{KeyStatus, Platform};

/// 调用各平台 API 验证 Key 是否有效
pub async fn check_key_validity(
    platform: &Platform,
    secret: &str,
    base_url: Option<&str>,
) -> KeyStatus {
    let result = match platform {
        Platform::Anthropic => {
            let url = base_url.unwrap_or("https://api.anthropic.com");
            check_anthropic(url, secret).await
        }
        Platform::Gemini => {
            let url = base_url.unwrap_or("https://generativelanguage.googleapis.com");
            check_gemini(url, secret).await
        }
        Platform::AwsBedrock
        | Platform::AzureOpenAI
        | Platform::HuggingFace
        | Platform::Replicate
        | Platform::Copilot
        | Platform::Auth0IDE => Ok(KeyStatus::Unknown),
        other => {
            let url = base_url
                .map(ToOwned::to_owned)
                .or_else(|| openai_like_base(other).map(ToOwned::to_owned));
            match url {
                Some(url) => check_openai_compatible(&url, secret).await,
                None => Ok(KeyStatus::Unknown),
            }
        }
    };

    result.unwrap_or(KeyStatus::Unknown)
}

fn openai_like_base(platform: &Platform) -> Option<&'static str> {
    match platform {
        Platform::OpenAI | Platform::Custom => Some("https://api.openai.com"),
        Platform::DeepSeek => Some("https://api.deepseek.com"),
        Platform::Aliyun => Some("https://dashscope.aliyuncs.com/compatible-mode"),
        Platform::Bytedance => Some("https://ark.cn-beijing.volces.com/api/v3"),
        Platform::Moonshot => Some("https://api.moonshot.cn"),
        Platform::Zhipu => Some("https://open.bigmodel.cn/api/paas/v4"),
        Platform::MiniMax => Some("https://api.minimax.chat/v1"),
        Platform::StepFun => Some("https://api.stepfun.com"),
        Platform::NvidiaNim => Some("https://integrate.api.nvidia.com"),
        Platform::SiliconFlow => Some("https://api.siliconflow.cn"),
        Platform::OpenRouter => Some("https://openrouter.ai/api"),
        Platform::Groq => Some("https://api.groq.com/openai"),
        Platform::Mistral => Some("https://api.mistral.ai"),
        Platform::XAi => Some("https://api.x.ai"),
        Platform::Cohere => Some("https://api.cohere.com/compatibility"),
        Platform::Perplexity => Some("https://api.perplexity.ai"),
        Platform::TogetherAi => Some("https://api.together.xyz"),
        Platform::Ollama => Some("http://127.0.0.1:11434"),
        Platform::Anthropic
        | Platform::Gemini
        | Platform::AwsBedrock
        | Platform::AzureOpenAI
        | Platform::HuggingFace
        | Platform::Replicate
        | Platform::Copilot
        | Platform::Auth0IDE => None,
    }
}

async fn check_openai_compatible(
    base_url: &str,
    secret: &str,
) -> Result<KeyStatus, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", secret))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    Ok(map_status_code(response.status().as_u16()))
}

async fn check_anthropic(base_url: &str, secret: &str) -> Result<KeyStatus, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

    let response = client
        .get(url)
        .header("x-api-key", secret)
        .header("anthropic-version", "2023-06-01")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    Ok(map_status_code(response.status().as_u16()))
}

async fn check_gemini(base_url: &str, secret: &str) -> Result<KeyStatus, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1beta/models?key={}",
        base_url.trim_end_matches('/'),
        secret
    );

    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    Ok(map_status_code(response.status().as_u16()))
}

fn map_status_code(status: u16) -> KeyStatus {
    match status {
        200 => KeyStatus::Valid,
        401 => KeyStatus::Invalid,
        403 => KeyStatus::Banned,
        429 => KeyStatus::RateLimit,
        _ => KeyStatus::Unknown,
    }
}
