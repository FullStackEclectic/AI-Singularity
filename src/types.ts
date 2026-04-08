// ─────────────────────────────────────────────────────────────────────────────
// Platform（API 供应商平台 — 与 Rust models.rs 保持同步）
// ─────────────────────────────────────────────────────────────────────────────

export type Platform =
  | "open_ai"
  | "anthropic"
  | "gemini"
  | "deep_seek"
  | "aliyun"
  | "bytedance"
  | "moonshot"
  | "zhipu"
  | "mini_max"
  | "step_fun"
  | "aws_bedrock"
  | "nvidia_nim"
  | "azure_open_a_i"
  | "silicon_flow"
  | "open_router"
  | "groq"
  | "mistral"
  | "x_ai"
  | "cohere"
  | "perplexity"
  | "together_ai"
  | "ollama"
  | "hugging_face"
  | "replicate"
  | "copilot"
  | "custom";

export const PLATFORM_LABELS: Record<Platform, string> = {
  open_ai:        "OpenAI",
  anthropic:      "Anthropic",
  gemini:         "Google Gemini",
  deep_seek:      "DeepSeek",
  aliyun:         "阿里云百炼",
  bytedance:      "字节豆包",
  moonshot:       "Moonshot (Kimi)",
  zhipu:          "智谱 GLM / Z.ai",
  mini_max:       "MiniMax",
  step_fun:       "StepFun",
  aws_bedrock:    "AWS Bedrock",
  nvidia_nim:     "NVIDIA NIM",
  azure_open_a_i: "Azure OpenAI",
  silicon_flow:   "SiliconFlow",
  open_router:    "OpenRouter",
  groq:           "Groq",
  mistral:        "Mistral AI",
  x_ai:           "xAI (Grok)",
  cohere:         "Cohere",
  perplexity:     "Perplexity",
  together_ai:    "Together AI",
  ollama:         "Ollama",
  hugging_face:   "Hugging Face",
  replicate:      "Replicate",
  copilot:        "GitHub Copilot",
  custom:         "自定义接口",
};

// ─────────────────────────────────────────────────────────────────────────────
// ToolTarget（同步目标工具 — 与 Rust models.rs 保持同步）
// ─────────────────────────────────────────────────────────────────────────────

export type ToolTarget =
  | "claude_code"
  | "codex"
  | "gemini_cli"
  | "open_code"
  | "open_claw"
  | "aider";

export const TOOL_TARGET_LABELS: Record<ToolTarget, string> = {
  claude_code: "Claude Code",
  codex:       "OpenAI Codex",
  gemini_cli:  "Gemini CLI",
  open_code:   "OpenCode",
  open_claw:   "OpenClaw",
  aider:       "Aider",
};

export const TOOL_TARGET_CONFIG_PATH: Record<ToolTarget, string> = {
  claude_code: "~/.claude/settings.json",
  codex:       "~/.codex/config.toml",
  gemini_cli:  "~/.gemini/settings.json",
  open_code:   "~/.config/opencode/opencode.json",
  open_claw:   "~/.openclaw/config.json",
  aider:       "~/.aider.conf.yml",
};

/** 向后兼容：旧 AiTool 类型别名 */
export type AiTool = ToolTarget;
export const AI_TOOL_LABELS = TOOL_TARGET_LABELS;

// ─────────────────────────────────────────────────────────────────────────────
// ProviderConfig
// ─────────────────────────────────────────────────────────────────────────────

export type ProviderCategory =
  | "official"
  | "cn_official"
  | "cloud_provider"
  | "aggregator"
  | "third_party"
  | "custom";

export const PROVIDER_CATEGORY_LABELS: Record<ProviderCategory, string> = {
  official:       "官方",
  cn_official:    "国内大模型",
  cloud_provider: "云厂商",
  aggregator:     "聚合平台",
  third_party:    "第三方中继",
  custom:         "自定义",
};

export interface ProviderConfig {
  id: string;
  name: string;
  platform: Platform;
  category?: ProviderCategory;
  base_url?: string;
  api_key_id?: string;
  model_name: string;
  is_active: boolean;
  /** JSON 数组字符串，如 '["claude_code","codex"]' */
  tool_targets?: string;
  icon?: string;
  icon_color?: string;
  website_url?: string;
  api_key_url?: string;
  notes?: string;
  extra_config?: string;
  sort_order?: number; // Sorting priority (smaller = first)
  created_at: string;
  updated_at: string;
}

/** 解析 tool_targets JSON 字符串为数组 */
export function parseToolTargets(provider: ProviderConfig): ToolTarget[] {
  try {
    if (!provider.tool_targets) return ["claude_code"];
    return JSON.parse(provider.tool_targets) as ToolTarget[];
  } catch {
    return ["claude_code"];
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// API Key
// ─────────────────────────────────────────────────────────────────────────────

export type KeyStatus = "unknown" | "valid" | "invalid" | "expired" | "banned" | "rate_limit";

export const STATUS_LABELS: Record<KeyStatus, string> = {
  unknown:    "未检测",
  valid:      "正常",
  invalid:    "无效",
  expired:    "已过期",
  banned:     "被封禁",
  rate_limit: "限速中",
};

export interface ApiKey {
  id: string;
  name: string;
  platform: Platform;
  base_url?: string;
  key_preview: string;
  status: KeyStatus;
  notes?: string;
  created_at: string;
  last_checked_at?: string;
  /** 轮询优先级：数值越大越优先（默认100） */
  priority: number;
  /** 用户自定义标签 */
  tags?: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Token 用量统计
// ─────────────────────────────────────────────────────────────────────────────

export interface TokenUsageStat {
  /** 分组名（client_app 或 model_name） */
  name: string;
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Balance / BalanceSummary
// ─────────────────────────────────────────────────────────────────────────────

export interface Balance {
  key_id: string;
  platform: Platform;
  balance_usd?: number;
  balance_cny?: number;
  total_usage_usd?: number;
  quota_remaining?: number;
  quota_reset_at?: string;
  synced_at: string;
}

export interface BalanceSummary {
  provider_id: string;
  provider_name: string;
  platform: string;
  latest_balance_usd?: number;
  latest_balance_cny?: number;
  quota_remaining?: number;
  quota_unit?: string;
  quota_reset_at?: string;
  last_updated?: string;
  low_balance_alert: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP Server
// ─────────────────────────────────────────────────────────────────────────────

export interface McpServer {
  id: string;
  name: string;
  command: string;
  args?: string;        // JSON array string
  env?: string;         // JSON map string
  description?: string;
  is_active: boolean;
  tool_targets?: string;
  created_at: string;
  updated_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Dashboard
// ─────────────────────────────────────────────────────────────────────────────

export interface DashboardStats {
  total_keys: number;
  valid_keys: number;
  invalid_keys: number;
  unknown_keys: number;
  total_platforms: number;
  total_cost_usd: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Misc
// ─────────────────────────────────────────────────────────────────────────────

export interface Model {
  id: string;
  name: string;
  platform: Platform;
  context_length?: number;
  supports_vision: boolean;
  supports_tools: boolean;
  input_price_per_1m?: number;
  output_price_per_1m?: number;
  is_available: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Environment Conflict Scan
// ─────────────────────────────────────────────────────────────────────────────

export interface EnvConflict {
  varName: string;
  varValue: string;
  sourceType: string;
  sourcePath: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// IDE 账号兵工厂 (降维池化指纹)
// ─────────────────────────────────────────────────────────────────────────────

export interface DeviceProfile {
  machine_id: string;
  mac_machine_id: string;
  dev_device_id: string;
  sqm_id: string;
}

export type AccountStatus = "active" | "expired" | "forbidden" | "rate_limited" | "unknown";

export interface OAuthToken {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  updated_at: string;
}

export interface IdeAccount {
  id: string;
  email: string;
  origin_platform: string;
  token: OAuthToken;
  status: AccountStatus;
  disabled_reason?: string;
  is_proxy_disabled: boolean;
  device_profile?: DeviceProfile;
  quota_json?: string;
  created_at: string;
  updated_at: string;
  last_used: string;
  /** 用户自定义标签 */
  tags?: string[];
  /** 用户自定义标注（备注名）*/
  label?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// 渠道专属模型配置（存储在 ProviderConfig.extra_config 字段中）
// ─────────────────────────────────────────────────────────────────────────────

export interface ClaudeToolConfig {
  /** 主模型 — 对应 ANTHROPIC_MODEL */
  model?: string;
  /** 推理模型 — 对应 ANTHROPIC_REASONING_MODEL */
  reasoningModel?: string;
  /** Haiku 小模型 — 对应 ANTHROPIC_DEFAULT_HAIKU_MODEL */
  haikuModel?: string;
  /** Sonnet 中模型 — 对应 ANTHROPIC_DEFAULT_SONNET_MODEL */
  sonnetModel?: string;
  /** Opus 大模型 — 对应 ANTHROPIC_DEFAULT_OPUS_MODEL */
  opusModel?: string;
}

export interface CodexToolConfig {
  /** 对应 codex config.toml 中的 model */
  model?: string;
  /** 推理强度 — "high" | "medium" | "low" */
  reasoningEffort?: string;
}

export interface GeminiToolConfig {
  /** 对应 GEMINI_MODEL */
  model?: string;
}

export interface OpenCodeToolConfig {
  model?: string;
}

export interface OpenClawToolConfig {
  model?: string;
}

export interface AiderToolConfig {
  model?: string;
}

/**
 * 渠道专属配置集合，序列化为 JSON 存入 ProviderConfig.extra_config
 * key 与 ToolTarget 保持一致
 */
export interface ToolSpecificConfigs {
  claude_code?: ClaudeToolConfig;
  codex?: CodexToolConfig;
  gemini_cli?: GeminiToolConfig;
  open_code?: OpenCodeToolConfig;
  open_claw?: OpenClawToolConfig;
  aider?: AiderToolConfig;
}

/** 从 ProviderConfig.extra_config 解析渠道专属配置 */
export function parseToolSpecificConfigs(provider: ProviderConfig): ToolSpecificConfigs {
  try {
    if (!provider.extra_config) return {};
    const parsed = JSON.parse(provider.extra_config);
    return (parsed.tool_configs as ToolSpecificConfigs) ?? {};
  } catch {
    return {};
  }
}

/** 将渠道专属配置序列化，合并进 extra_config JSON 字符串 */
export function serializeToolSpecificConfigs(
  existingExtraConfig: string | undefined,
  configs: ToolSpecificConfigs,
): string {
  let base: Record<string, unknown> = {};
  try {
    if (existingExtraConfig) base = JSON.parse(existingExtraConfig);
  } catch { /* ignore */ }
  return JSON.stringify({ ...base, tool_configs: configs });
}
