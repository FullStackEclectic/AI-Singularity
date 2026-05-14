use crate::proxy::converter::OpenAIRequest;
use crate::proxy::mappers::{MapperChunk, ProtocolMapper};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GeminiMapper;

#[async_trait]
impl ProtocolMapper for GeminiMapper {
    type Request = OpenAIRequest;

    fn get_protocol() -> String {
        "gemini".to_string()
    }

    fn get_model(req: &Self::Request) -> &str {
        &req.model
    }

    fn initial_chunks() -> Vec<MapperChunk> {
        vec![]
    }

    /// 将 Gemini SSE chunk JSON 转换为 OpenAI chat.completion.chunk 格式。
    ///
    /// Gemini SSE 数据格式：
    /// ```json
    /// {"candidates":[{"content":{"parts":[{"text":"..."}]},"finishReason":"STOP"}]}
    /// ```
    ///
    /// 输出 OpenAI 格式：
    /// ```json
    /// {"id":"...","object":"chat.completion.chunk","created":...,"model":"...","choices":[{"index":0,"delta":{"content":"..."},"finish_reason":null}]}
    /// ```
    async fn map_delta(
        model: &str,
        delta: String,
        is_final: bool,
        _tool_call_buffer: &mut String,
        _in_tool_call: &mut bool,
        _tool_call_index: &mut u32,
    ) -> Result<Vec<MapperChunk>> {
        if is_final {
            // 发送终止帧（空 delta + finish_reason: stop）
            let chunk = make_chunk(model, "", true)?;
            return Ok(vec![MapperChunk {
                event: None,
                data: chunk,
            }]);
        }

        if delta.is_empty() {
            return Ok(vec![]);
        }

        // 尝试解析 Gemini SSE JSON
        let json_val: serde_json::Value = match serde_json::from_str(&delta) {
            Ok(v) => v,
            Err(_) => {
                // 无法解析时透传原始文本
                return Ok(vec![MapperChunk {
                    event: None,
                    data: make_chunk(model, &delta, false)?,
                }]);
            }
        };

        // 提取 candidates[0].content.parts[0].text
        let text = json_val["candidates"]
            .as_array()
            .and_then(|c| c.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|p| p.first())
            .and_then(|p| p["text"].as_str())
            .unwrap_or("");

        if text.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![MapperChunk {
            event: None,
            data: make_chunk(model, text, false)?,
        }])
    }
}

fn make_chunk(model: &str, content: &str, is_final: bool) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let chunk = json!({
        "id": format!("chatcmpl-gemini-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion.chunk",
        "created": now,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": if is_final { json!({}) } else { json!({ "content": content }) },
            "finish_reason": if is_final { json!("stop") } else { serde_json::Value::Null }
        }]
    });
    Ok(chunk.to_string())
}
