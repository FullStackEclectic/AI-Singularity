use crate::models::SpeedTestResult;
use std::time::Instant;
use tracing::info;

/// 所有已知的平台端点（platform_key, display_name, endpoint）
pub const KNOWN_ENDPOINTS: &[(&str, &str, &str)] = &[
    ("open_ai", "OpenAI", "https://api.openai.com/v1/models"),
    (
        "anthropic",
        "Anthropic",
        "https://api.anthropic.com/v1/models",
    ),
    (
        "gemini",
        "Google Gemini",
        "https://generativelanguage.googleapis.com/v1/models",
    ),
    (
        "deep_seek",
        "DeepSeek",
        "https://api.deepseek.com/v1/models",
    ),
    (
        "aliyun",
        "阿里云百炼",
        "https://dashscope.aliyuncs.com/compatible-mode/v1/models",
    ),
    (
        "bytedance",
        "字节豆包",
        "https://ark.cn-beijing.volces.com/api/v3/models",
    ),
    (
        "moonshot",
        "Moonshot (Kimi)",
        "https://api.moonshot.cn/v1/models",
    ),
    (
        "zhipu",
        "智谱 GLM",
        "https://open.bigmodel.cn/api/paas/v4/models",
    ),
    (
        "nvidia_nim",
        "NVIDIA NIM",
        "https://integrate.api.nvidia.com/v1/models",
    ),
];

pub struct SpeedTestService;

impl SpeedTestService {
    /// Test a single endpoint — fire a GET request, no auth, just measure latency
    pub async fn test_endpoint(
        platform: &str,
        display_name: &str,
        endpoint: &str,
    ) -> SpeedTestResult {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();

        let start = Instant::now();
        let result = client.get(endpoint).send().await;
        let elapsed = start.elapsed().as_millis() as u64;

        let (latency_ms, status) = match result {
            Ok(resp) => {
                info!(
                    "SpeedTest {}: {}ms (HTTP {})",
                    display_name,
                    elapsed,
                    resp.status()
                );
                // any response (even 401) means the server is reachable
                (Some(elapsed), "ok".to_string())
            }
            Err(e) if e.is_timeout() => (None, "timeout".to_string()),
            Err(_) => (None, "error".to_string()),
        };

        SpeedTestResult {
            platform: platform.to_string(),
            endpoint: endpoint.to_string(),
            latency_ms,
            status,
        }
    }

    /// Test all known endpoints concurrently
    pub async fn test_all() -> Vec<SpeedTestResult> {
        let futures: Vec<_> = KNOWN_ENDPOINTS
            .iter()
            .map(|(key, name, endpoint)| Self::test_endpoint(key, name, endpoint))
            .collect();

        // Run all concurrently
        futures::future::join_all(futures).await
    }
}
