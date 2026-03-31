/// 协议转换：OpenAI ↔ Anthropic ↔ Gemini
/// 负责把各平台专有格式互相转换，使代理可以统一接受 OpenAI 格式并转发到任意平台
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 通用消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// OpenAI 格式请求体（/v1/chat/completions）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI → Anthropic 格式转换
pub fn openai_to_anthropic(req: &OpenAIRequest) -> Value {
    // 提取 system prompt
    let system_prompt: Option<String> = req.messages.iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());

    // 过滤掉 system messages，只保留 user/assistant
    let messages: Vec<Value> = req.messages.iter()
        .filter(|m| m.role != "system")
        .map(|m| json!({
            "role": m.role,
            "content": m.content,
        }))
        .collect();

    let mut body = json!({
        "model": req.model,
        "messages": messages,
        "max_tokens": req.max_tokens.unwrap_or(4096),
    });

    if let Some(sys) = system_prompt {
        body["system"] = json!(sys);
    }
    if let Some(temp) = req.temperature {
        body["temperature"] = json!(temp);
    }
    if let Some(stream) = req.stream {
        body["stream"] = json!(stream);
    }

    body
}

/// Anthropic → OpenAI 响应格式转换
pub fn anthropic_to_openai_response(anthropic_resp: &Value) -> Value {
    let content = anthropic_resp["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");

    let input_tokens = anthropic_resp["usage"]["input_tokens"].as_u64().unwrap_or(0);
    let output_tokens = anthropic_resp["usage"]["output_tokens"].as_u64().unwrap_or(0);

    json!({
        "id": anthropic_resp["id"],
        "object": "chat.completion",
        "model": anthropic_resp["model"],
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens,
        }
    })
}

/// OpenAI → Gemini 格式转换（/v1beta/models/{model}:generateContent）
pub fn openai_to_gemini(req: &OpenAIRequest) -> (String, Value) {
    // 提取 system instruction
    let system_instruction = req.messages.iter()
        .find(|m| m.role == "system")
        .map(|m| json!({ "parts": [{ "text": m.content }] }));

    let contents: Vec<Value> = req.messages.iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            let role = if m.role == "assistant" { "model" } else { "user" };
            json!({
                "role": role,
                "parts": [{ "text": m.content }],
            })
        })
        .collect();

    let mut body = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": req.max_tokens.unwrap_or(4096),
        }
    });

    if let Some(sys) = system_instruction {
        body["systemInstruction"] = sys;
    }
    if let Some(temp) = req.temperature {
        body["generationConfig"]["temperature"] = json!(temp);
    }

    // model 字符串（去掉可能的前缀）
    let model = req.model.replace("models/", "");
    (model, body)
}

/// Gemini → OpenAI 响应格式转换
pub fn gemini_to_openai_response(gemini_resp: &Value, model: &str) -> Value {
    let content = gemini_resp["candidates"]
        .as_array()
        .and_then(|c| c.first())
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|p| p.first())
        .and_then(|p| p["text"].as_str())
        .unwrap_or("");

    let prompt_tokens = gemini_resp["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0);
    let output_tokens = gemini_resp["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0);

    json!({
        "id": format!("gemini-{}", chrono::Utc::now().timestamp()),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": prompt_tokens + output_tokens,
        }
    })
}
