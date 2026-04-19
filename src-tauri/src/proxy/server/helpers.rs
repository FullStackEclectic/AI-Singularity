use crate::models::Platform;
use crate::proxy::converter::OpenAIMessage;
use serde_json::{json, Value};

pub(super) fn get_platform_for_model(model: &str) -> Option<&'static str> {
    let lower = model.to_lowercase();
    if lower.contains("claude") {
        Some("anthropic")
    } else if lower.contains("gemini") {
        Some("gemini")
    } else if lower.contains("gpt") || lower.contains("o1") || lower.contains("o3") {
        Some("openai")
    } else if lower.contains("deepseek") {
        Some("deep_seek")
    } else if lower.contains("qwen") || lower.contains("bailian") {
        Some("aliyun")
    } else if lower.contains("moonshot") {
        Some("moonshot")
    } else if lower.contains("glm") {
        Some("zhipu")
    } else if lower.contains("doubao") {
        Some("bytedance")
    } else {
        None
    }
}

pub(super) fn get_fallback_model(model: &str) -> Option<&'static str> {
    let lower = model.to_lowercase();
    if lower.contains("claude-3-5") || lower.contains("claude-3-7") {
        Some("gemini-1.5-flash")
    } else if lower.contains("claude") {
        Some("gemini-1.5-pro")
    } else if lower.contains("gpt-4")
        || lower.contains("gpt-4o")
        || lower.contains("o1")
        || lower.contains("o3")
    {
        Some("gemini-1.5-pro")
    } else if lower.contains("gpt-3.5") || lower.contains("gpt-4o-mini") {
        Some("gemini-1.5-flash")
    } else if lower.contains("deepseek") {
        Some("gemini-1.5-flash")
    } else {
        None
    }
}

pub(super) fn platform_base_url(platform: &Platform) -> &'static str {
    match platform {
        Platform::OpenAI => "https://api.openai.com",
        Platform::DeepSeek => "https://api.deepseek.com",
        Platform::Aliyun => "https://dashscope.aliyuncs.com/compatible-mode",
        Platform::Moonshot => "https://api.moonshot.cn",
        Platform::Zhipu => "https://open.bigmodel.cn/api/paas",
        Platform::Bytedance => "https://ark.cn-beijing.volces.com/api",
        _ => "https://api.openai.com",
    }
}

pub(super) fn extract_image_prompt(messages: &[OpenAIMessage]) -> Option<String> {
    if let Some(msg) = messages.last() {
        if msg.role == "user" {
            let content = msg.content.trim().to_lowercase();
            if content.starts_with("draw a picture of") {
                return Some(msg.content[17..].trim().to_string());
            } else if content.starts_with("画一张") {
                return Some(msg.content[9..].trim().to_string());
            } else if content.starts_with("生成图片") {
                return Some(msg.content[12..].trim().to_string());
            }
        }
    }
    None
}

pub(super) fn construct_pollinations_response(prompt: &str) -> Value {
    let encoded: String = url::form_urlencoded::byte_serialize(prompt.as_bytes()).collect();
    let img_url = format!(
        "https://image.pollinations.ai/prompt/{}?width=1024&height=1024&nologo=true",
        encoded
    );
    json!({
        "id": format!("pollinations-{}", chrono::Utc::now().timestamp()),
        "object": "chat.completion",
        "model": "pollinations-free",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Here is your generated image via free proxy:\n\n![Generated Image]({})", img_url),
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        }
    })
}
