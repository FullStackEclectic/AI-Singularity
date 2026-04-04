use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
use crate::proxy::mappers::{ProtocolMapper, MapperChunk};
use crate::proxy::converter::OpenAIRequest; // 这里虽然叫 OpenAIRequest，其实是代理网关统一入口的格式

pub struct AnthropicMapper;

#[async_trait]
impl ProtocolMapper for AnthropicMapper {
    type Request = OpenAIRequest;

    fn get_protocol() -> String {
        "anthropic".to_string()
    }

    fn get_model(req: &Self::Request) -> &str {
        &req.model
    }

    fn initial_chunks() -> Vec<MapperChunk> {
        vec![
            MapperChunk {
                event: Some("message_start".into()),
                data: r#"{"type":"message_start","message":{"id":"msg_proxy","type":"message","role":"assistant","model":"claude-3-op","content":[],"usage":{"input_tokens":0,"output_tokens":0}}}"#.into(),
            },
            MapperChunk {
                event: Some("content_block_start".into()),
                data: r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#.into(),
            },
        ]
    }

    async fn map_delta(
        _model: &str,
        delta: String,
        is_final: bool,
        tool_call_buffer: &mut String,
        in_tool_call: &mut bool,
        tool_call_index: &mut u32,
    ) -> Result<Vec<MapperChunk>> {
        let mut results = vec![];

        if is_final {
            results.push(MapperChunk { event: Some("content_block_stop".into()), data: format!(r#"{{"type":"content_block_stop","index":{}}}"#, *tool_call_index * 2) });
            
            let stop_reason = if *tool_call_index > 0 { "tool_use" } else { "end_turn" };
            let message_delta = format!(r#"{{"type":"message_delta","delta":{{"stop_reason":"{}","stop_sequence":null}},"usage":{{"output_tokens":0}}}}"#, stop_reason);
            results.push(MapperChunk { event: Some("message_delta".into()), data: message_delta });
            results.push(MapperChunk { event: Some("message_stop".into()), data: r#"{"type":"message_stop"}"#.into() });
            return Ok(results);
        }

        let mut pending_text = delta;
        while !pending_text.is_empty() {
            if !*in_tool_call {
                if let Some(start_idx) = pending_text.find("<tool_call>") {
                    let before_text = &pending_text[..start_idx];
                    let current_text_index = *tool_call_index * 2;
                    if !before_text.is_empty() {
                        let delta_json = json!({ "type": "content_block_delta", "index": current_text_index, "delta": { "type": "text_delta", "text": before_text } });
                        results.push(MapperChunk { event: Some("content_block_delta".into()), data: delta_json.to_string() });
                    }
                    *in_tool_call = true;
                    results.push(MapperChunk { event: Some("content_block_stop".into()), data: format!(r#"{{"type":"content_block_stop","index":{}}}"#, current_text_index) });

                    pending_text = pending_text[start_idx + "<tool_call>".len()..].to_string();
                } else {
                    let text_index = *tool_call_index * 2;
                    let delta_json = json!({ "type": "content_block_delta", "index": text_index, "delta": { "type": "text_delta", "text": pending_text } });
                    results.push(MapperChunk { event: Some("content_block_delta".into()), data: delta_json.to_string() });
                    pending_text = String::new();
                }
            } else {
                if let Some(end_idx) = pending_text.find("</tool_call>") {
                    let inner_text = &pending_text[..end_idx];
                    tool_call_buffer.push_str(inner_text);
                    
                    let trim_buf = tool_call_buffer.trim();
                    let tool_idx = *tool_call_index * 2 + 1; // Tool block index
                    let next_text_idx = *tool_call_index * 2 + 2; // Next text block index

                    if !trim_buf.is_empty() {
                        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(trim_buf) {
                            let name = json_obj.get("name").and_then(|v| v.as_str()).unwrap_or("unknown_tool").to_string();
                            let args = json_obj.get("arguments").cloned().unwrap_or_else(|| json!({}));
                            
                            results.push(MapperChunk {
                                event: Some("content_block_start".into()),
                                data: json!({ "type": "content_block_start", "index": tool_idx, "content_block": { "type": "tool_use", "id": format!("toolu_proxy_{}", *tool_call_index), "name": name, "input": {} } }).to_string(),
                            });
                            
                            let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                            results.push(MapperChunk {
                                event: Some("content_block_delta".into()),
                                data: json!({ "type": "content_block_delta", "index": tool_idx, "delta": { "type": "input_json_delta", "partial_json": args_str } }).to_string(),
                            });
                            
                            results.push(MapperChunk {
                                event: Some("content_block_stop".into()),
                                data: json!({ "type": "content_block_stop", "index": tool_idx }).to_string(),
                            });
                            *tool_call_index += 1;
                        } else {
                            let fallback = format!("<tool_call>{}</tool_call>", trim_buf);
                            results.push(MapperChunk {
                                event: Some("content_block_delta".into()),
                                data: json!({ "type": "content_block_delta", "index": tool_idx, "delta": { "type": "text_delta", "text": fallback } }).to_string()
                            });
                        }
                    }

                    pending_text = pending_text[end_idx + "</tool_call>".len()..].to_string();
                    *in_tool_call = false;
                    tool_call_buffer.clear();
                    
                    // Open the next text block
                    results.push(MapperChunk { event: Some("content_block_start".into()), data: format!(r#"{{"type":"content_block_start","index":{},"content_block":{{"type":"text","text":""}}}}"#, next_text_idx) });
                } else {
                    tool_call_buffer.push_str(&pending_text);
                    pending_text = String::new();
                }
            }
        }
        Ok(results)
    }
}
