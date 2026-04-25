/// 协议转换：OpenAI ↔ Anthropic ↔ Gemini
/// 负责把各平台专有格式互相转换，使代理可以统一接受 OpenAI 格式并转发到任意平台
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 通用消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
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
    let system_prompt: Option<String> = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());

    // 过滤掉 system messages，只保留 user/assistant
    let messages: Vec<Value> = req
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            json!({
                "role": m.role,
                "content": m.content,
            })
        })
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

    let input_tokens = anthropic_resp["usage"]["input_tokens"]
        .as_u64()
        .unwrap_or(0);
    let output_tokens = anthropic_resp["usage"]["output_tokens"]
        .as_u64()
        .unwrap_or(0);

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
    let system_instruction = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| json!({ "parts": [{ "text": m.content }] }));

    let contents: Vec<Value> = req
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            let role = if m.role == "assistant" {
                "model"
            } else {
                "user"
            };
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

    let prompt_tokens = gemini_resp["usageMetadata"]["promptTokenCount"]
        .as_u64()
        .unwrap_or(0);
    let output_tokens = gemini_resp["usageMetadata"]["candidatesTokenCount"]
        .as_u64()
        .unwrap_or(0);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(with_system: bool) -> OpenAIRequest {
        let mut messages = vec![];
        if with_system {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: "你是一个助手".to_string(),
            });
        }
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: "你好".to_string(),
        });
        messages.push(OpenAIMessage {
            role: "assistant".to_string(),
            content: "你好，有什么可以帮你的？".to_string(),
        });
        OpenAIRequest {
            model: "gpt-4o".to_string(),
            messages,
            max_tokens: Some(512),
            temperature: Some(0.7),
            stream: Some(true),
        }
    }

    #[test]
    fn openai_to_anthropic_extracts_system_prompt() {
        let req = sample_request(true);
        let body = openai_to_anthropic(&req);

        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["system"], "你是一个助手");
        assert_eq!(body["max_tokens"], 512);
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["stream"], true);

        // system 应被剔除，messages 仅剩 user / assistant
        let messages = body["messages"].as_array().expect("messages 数组");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "assistant");
    }

    #[test]
    fn openai_to_anthropic_without_system_uses_default_max_tokens() {
        let mut req = sample_request(false);
        req.max_tokens = None;
        let body = openai_to_anthropic(&req);
        assert!(body.get("system").is_none());
        assert_eq!(body["max_tokens"], 4096);
    }

    #[test]
    fn anthropic_to_openai_response_maps_usage_and_content() {
        let anthropic_resp = json!({
            "id": "msg_01",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "hello"}],
            "usage": {"input_tokens": 10, "output_tokens": 7}
        });
        let resp = anthropic_to_openai_response(&anthropic_resp);

        assert_eq!(resp["id"], "msg_01");
        assert_eq!(resp["object"], "chat.completion");
        assert_eq!(resp["choices"][0]["message"]["content"], "hello");
        assert_eq!(resp["choices"][0]["finish_reason"], "stop");
        assert_eq!(resp["usage"]["prompt_tokens"], 10);
        assert_eq!(resp["usage"]["completion_tokens"], 7);
        assert_eq!(resp["usage"]["total_tokens"], 17);
    }

    #[test]
    fn openai_to_gemini_remaps_assistant_role_to_model() {
        let req = sample_request(true);
        let (model, body) = openai_to_gemini(&req);
        assert_eq!(model, "gpt-4o");

        let contents = body["contents"].as_array().expect("contents 数组");
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model"); // assistant 映射为 model

        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "你是一个助手");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 512);
        assert_eq!(body["generationConfig"]["temperature"], 0.7);
    }

    #[test]
    fn openai_to_gemini_strips_models_prefix() {
        let mut req = sample_request(false);
        req.model = "models/gemini-2.0-flash".to_string();
        let (model, _) = openai_to_gemini(&req);
        assert_eq!(model, "gemini-2.0-flash");
    }

    #[test]
    fn gemini_to_openai_response_handles_missing_usage_gracefully() {
        let gemini_resp = json!({
            "candidates": [{"content": {"parts": [{"text": "hi"}]}}]
        });
        let resp = gemini_to_openai_response(&gemini_resp, "gemini-2.0-flash");
        assert_eq!(resp["choices"][0]["message"]["content"], "hi");
        assert_eq!(resp["usage"]["prompt_tokens"], 0);
        assert_eq!(resp["usage"]["completion_tokens"], 0);
        assert_eq!(resp["usage"]["total_tokens"], 0);
    }

    #[test]
    fn gemini_to_openai_response_extracts_usage_when_present() {
        let gemini_resp = json!({
            "candidates": [{"content": {"parts": [{"text": "hi"}]}}],
            "usageMetadata": {"promptTokenCount": 12, "candidatesTokenCount": 5}
        });
        let resp = gemini_to_openai_response(&gemini_resp, "gemini-2.0-flash");
        assert_eq!(resp["usage"]["prompt_tokens"], 12);
        assert_eq!(resp["usage"]["completion_tokens"], 5);
        assert_eq!(resp["usage"]["total_tokens"], 17);
    }
}
