use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::proxy::mappers::{ProtocolMapper, MapperChunk};
use crate::proxy::converter::OpenAIRequest;

pub struct OpenAiMapper;

#[async_trait]
impl ProtocolMapper for OpenAiMapper {
    type Request = OpenAIRequest;

    fn get_protocol() -> String {
        "openai".to_string()
    }

    fn get_model(req: &Self::Request) -> &str {
        &req.model
    }

    async fn map_delta(
        model: &str,
        delta: String,
        is_final: bool,
        tool_call_buffer: &mut String,
        in_tool_call: &mut bool,
        tool_call_index: &mut u32,
    ) -> Result<Vec<MapperChunk>> {
        let mut results = vec![];
        
        if is_final {
            results.push(MapperChunk { event: None, data: generate_chunk(model, "", true)? });
            return Ok(results);
        }

        if delta.is_empty() {
            return Ok(results);
        }

        let mut pending_text = delta;
        while !pending_text.is_empty() {
            if !*in_tool_call {
                if let Some(start_pos) = pending_text.find("<tool_call>") {
                    *in_tool_call = true;
                    let prefix = &pending_text[..start_pos];
                    if !prefix.is_empty() {
                        results.push(MapperChunk { event: None, data: generate_chunk(model, prefix, false)? });
                    }
                    pending_text = pending_text[start_pos + "<tool_call>".len()..].to_string();
                } else {
                    results.push(MapperChunk { event: None, data: generate_chunk(model, &pending_text, false)? });
                    pending_text = String::new();
                }
            } else {
                if let Some(end_pos) = pending_text.find("</tool_call>") {
                    let inner_text = &pending_text[..end_pos];
                    tool_call_buffer.push_str(inner_text);
                    let trim_buf = tool_call_buffer.trim();
                    if !trim_buf.is_empty() {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trim_buf) {
                            let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("unknown_tool").to_string();
                            let args = v.get("arguments").map(|a| if let Some(s) = a.as_str() { s.to_string() } else { a.to_string() }).unwrap_or_else(|| "{}".to_string());
                            results.push(MapperChunk { event: None, data: generate_tool_call_chunk(model, &name, &args, *tool_call_index)? });
                            *tool_call_index += 1;
                        } else {
                            let fallback = format!("<tool_call>{}</tool_call>", trim_buf);
                            results.push(MapperChunk { event: None, data: generate_chunk(model, &fallback, false)? });
                        }
                    }
                    tool_call_buffer.clear();
                    *in_tool_call = false;
                    pending_text = pending_text[end_pos + "</tool_call>".len()..].to_string();
                } else {
                    tool_call_buffer.push_str(&pending_text);
                    pending_text = String::new();
                }
            }
        }

        Ok(results)
    }
}

fn generate_chunk(model: &str, content: &str, is_final: bool) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let chunk = json!({
        "id": format!("chatcmpl-proxy-{}", uuid::Uuid::new_v4()),
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

fn generate_tool_call_chunk(model: &str, name: &str, args: &str, tool_call_index: u32) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let chunk = json!({
        "id": format!("chatcmpl-proxy-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion.chunk",
        "created": now,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": tool_call_index,
                    "id": format!("call_{}_{}", uuid::Uuid::new_v4().to_string().replace("-", ""), tool_call_index),
                    "type": "function",
                    "function": { "name": name, "arguments": args }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });
    Ok(chunk.to_string())
}
