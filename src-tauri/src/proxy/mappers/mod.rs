#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

pub mod anthropic;
pub mod openai;

/// 映射后的增量响应块
#[derive(Debug, Clone)]
pub struct MapperChunk {
    pub event: Option<String>,
    pub data: String,
}

/// 发送给客户端的流式结尾统计数据
#[derive(Debug, Clone, Default)]
pub struct StreamMetadata {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub error: Option<String>,
}

/// 协议转换器 Trait，用于将各种 API 协议映射到底层的 Target 服务上
#[async_trait]
pub trait ProtocolMapper: Send + Sync + 'static {
    /// 请求类型
    type Request: DeserializeOwned + Send + Sync + 'static;

    /// 获取协议标识符
    fn get_protocol() -> String;

    /// 从请求中提取要使用的平台或模型标识
    fn get_model(req: &Self::Request) -> &str;

    /// 处理流式返回：将底层的增量文本 (Delta)，转换为符合特定协议规范的流式响应格式（Chunk）
    /// - `model`: 模型名称
    /// - `delta`: 当前接收到的文本块
    /// - `is_final`: 是否为流的尽头
    /// - `tool_call_buffer`: 供调用方暂存工具调用拼凑的 Buffer（若底层返回的是分段的 JSON，则需拼凑后再外发）
    /// - `in_tool_call`: 状态机，标识当前流是否陷入工具函数调用阶段
    /// - `tool_call_index`: 当前调用的工具索引号
    async fn map_delta(
        model: &str,
        delta: String,
        is_final: bool,
        tool_call_buffer: &mut String,
        in_tool_call: &mut bool,
        tool_call_index: &mut u32,
    ) -> Result<Vec<MapperChunk>>;

    /// 返回流式连接建立之初需要打出去的初始帧（如 content_block_start 或者 role: assistant）
    fn initial_chunks() -> Vec<MapperChunk> {
        vec![]
    }
}
