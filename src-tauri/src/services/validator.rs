use crate::models::{KeyStatus, Platform};

/// 调用各平台API验证Key是否有效
pub async fn check_key_validity(
    platform: &Platform,
    secret: &str,
    base_url: Option<&str>,
) -> KeyStatus {
    let result = match platform {
        Platform::OpenAI | Platform::Custom => {
            let url = base_url.unwrap_or("https://api.openai.com");
            check_openai_compatible(url, secret).await
        }
        Platform::Anthropic => check_anthropic(secret).await,
        Platform::DeepSeek => check_openai_compatible("https://api.deepseek.com", secret).await,
        Platform::Aliyun => {
            check_openai_compatible("https://dashscope.aliyuncs.com/compatible-mode", secret).await
        }
        Platform::Moonshot => check_openai_compatible("https://api.moonshot.cn", secret).await,
        Platform::Zhipu => {
            check_openai_compatible("https://open.bigmodel.cn/api/paas", secret).await
        }
        // 其他平台暂未实现
        _ => Ok(KeyStatus::Unknown),
    };

    result.unwrap_or(KeyStatus::Unknown)
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

    Ok(match response.status().as_u16() {
        200 => KeyStatus::Valid,
        401 => KeyStatus::Invalid,
        403 => KeyStatus::Banned,
        429 => KeyStatus::RateLimit,
        _ => KeyStatus::Unknown,
    })
}

async fn check_anthropic(secret: &str) -> Result<KeyStatus, reqwest::Error> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", secret)
        .header("anthropic-version", "2023-06-01")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    Ok(match response.status().as_u16() {
        200 => KeyStatus::Valid,
        401 => KeyStatus::Invalid,
        403 => KeyStatus::Banned,
        429 => KeyStatus::RateLimit,
        _ => KeyStatus::Unknown,
    })
}
