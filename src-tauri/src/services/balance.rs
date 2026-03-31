use crate::models::{Balance, Platform};
use crate::AppError;
use chrono::Utc;

/// 调用各平台 API 获取余额信息
pub async fn fetch_balance(platform: &Platform, secret: &str) -> Result<Balance, AppError> {
    let balance = match platform {
        Platform::OpenAI => fetch_openai_balance(secret).await?,
        Platform::DeepSeek => fetch_deepseek_balance(secret).await?,
        Platform::Moonshot => fetch_moonshot_balance(secret).await?,
        Platform::Aliyun => fetch_aliyun_balance(secret).await?,
        // Anthropic / Gemini 无公开余额 API
        _ => Balance {
            key_id: String::new(),
            platform: platform.clone(),
            balance_usd: None,
            balance_cny: None,
            total_usage_usd: None,
            quota_remaining: None,
            quota_reset_at: None,
            synced_at: Utc::now(),
        },
    };
    Ok(balance)
}

async fn fetch_openai_balance(secret: &str) -> Result<Balance, AppError> {
    let client = reqwest::Client::new();

    // OpenAI 信用额度查询
    let resp = client
        .get("https://api.openai.com/dashboard/billing/credit_grants")
        .header("Authorization", format!("Bearer {}", secret))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if resp.status() == 401 {
        return Err(AppError::InvalidApiKey);
    }

    let json: serde_json::Value = resp.json().await?;
    let total_granted = json["total_granted"].as_f64().unwrap_or(0.0);
    let total_used = json["total_used"].as_f64().unwrap_or(0.0);
    let remaining = total_granted - total_used;

    Ok(Balance {
        key_id: String::new(),
        platform: Platform::OpenAI,
        balance_usd: Some(remaining),
        balance_cny: None,
        total_usage_usd: Some(total_used),
        quota_remaining: Some(remaining),
        quota_reset_at: None,
        synced_at: Utc::now(),
    })
}

async fn fetch_deepseek_balance(secret: &str) -> Result<Balance, AppError> {
    let client = reqwest::Client::new();

    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {}", secret))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if resp.status() == 401 {
        return Err(AppError::InvalidApiKey);
    }

    let json: serde_json::Value = resp.json().await?;
    // DeepSeek 返回格式：{"balance_infos": [{"currency": "CNY", "total_balance": "xx", ...}]}
    let balance_infos = json["balance_infos"].as_array();
    let (cny, usd) = if let Some(infos) = balance_infos {
        let cny = infos.iter()
            .find(|i| i["currency"].as_str() == Some("CNY"))
            .and_then(|i| i["total_balance"].as_str())
            .and_then(|s| s.parse::<f64>().ok());
        let usd = infos.iter()
            .find(|i| i["currency"].as_str() == Some("USD"))
            .and_then(|i| i["total_balance"].as_str())
            .and_then(|s| s.parse::<f64>().ok());
        (cny, usd)
    } else {
        (None, None)
    };

    Ok(Balance {
        key_id: String::new(),
        platform: Platform::DeepSeek,
        balance_usd: usd,
        balance_cny: cny,
        total_usage_usd: None,
        quota_remaining: usd.or(cny),
        quota_reset_at: None,
        synced_at: Utc::now(),
    })
}

async fn fetch_moonshot_balance(secret: &str) -> Result<Balance, AppError> {
    let client = reqwest::Client::new();

    let resp = client
        .get("https://api.moonshot.cn/v1/users/me/balance")
        .header("Authorization", format!("Bearer {}", secret))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if resp.status() == 401 {
        return Err(AppError::InvalidApiKey);
    }

    let json: serde_json::Value = resp.json().await?;
    // Moonshot 返回格式：{"data": {"available_balance": 100.0, "cash_balance": 80.0}}
    let available = json["data"]["available_balance"].as_f64();
    let cash = json["data"]["cash_balance"].as_f64();

    Ok(Balance {
        key_id: String::new(),
        platform: Platform::Moonshot,
        balance_usd: None,
        balance_cny: available,
        total_usage_usd: None,
        quota_remaining: cash,
        quota_reset_at: None,
        synced_at: Utc::now(),
    })
}

async fn fetch_aliyun_balance(secret: &str) -> Result<Balance, AppError> {
    let client = reqwest::Client::new();

    // 阿里云百炼无标准余额 API，调用模型列表接口验证有效性
    let resp = client
        .get("https://dashscope.aliyuncs.com/compatible-mode/v1/models")
        .header("Authorization", format!("Bearer {}", secret))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if resp.status() == 401 {
        return Err(AppError::InvalidApiKey);
    }

    // 阿里云暂不支持余额查询，仅记录同步时间
    Ok(Balance {
        key_id: String::new(),
        platform: Platform::Aliyun,
        balance_usd: None,
        balance_cny: None,
        total_usage_usd: None,
        quota_remaining: None,
        quota_reset_at: None,
        synced_at: Utc::now(),
    })
}
