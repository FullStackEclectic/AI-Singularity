/**
 * AI Singularity — Provider 预设库
 *
 * 每个终端预设分三类：
 *   1. AI Singularity 高速中转（推荐）
 *   2. 官方原厂直连
 *   3. 主流第三方平台直连（DeepSeek / Moonshot / SiliconFlow / OpenRouter / 阿里云 / 字节豆包）
 */

import type { ToolTarget } from "../types";

export type ProviderCategory =
  | "relay"    // AI Singularity 官方中转（强推）
  | "official" // 官方原厂直连
  | "custom"   // 用户自定义

export interface TemplateVar {
  key: string;
  label: string;
  placeholder: string;
  defaultValue?: string;
}

export interface ProviderPreset {
  presetId: string;
  name: string;
  category: ProviderCategory;
  toolTargets?: ToolTarget[];
  platform?: "open_ai" | "anthropic" | "gemini" | "custom" | "ollama" | "azure_open_a_i" | "aws_bedrock";
  icon?: string;
  iconColor?: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  defaultBaseUrl?: string;
  defaultModel?: string;
  templateVars?: TemplateVar[];
  endpointCandidates?: string[];
  notes?: string;
  settingsConfig?: any;
}

// ─────────────────────────────────────────────────────────────────────────────
// 预设数据
// ─────────────────────────────────────────────────────────────────────────────

export const PROVIDER_PRESETS: ProviderPreset[] = [
  // ── Claude Code ──────────────────────────────────────────────────────────
  {
    presetId: "cc-anthropic-official",
    name: "Anthropic (官方直连)",
    category: "official",
    toolTargets: ["claude_code"],
    platform: "anthropic",
    icon: "anthropic",
    iconColor: "#D97757",
    websiteUrl: "https://www.anthropic.com/claude-code",
    apiKeyUrl: "https://console.anthropic.com/settings/keys",
    defaultBaseUrl: "https://api.anthropic.com",
    defaultModel: "claude-opus-4-5",
    notes: "直连 Anthropic 官方 API，需要境外的网络环境。",
  },
  {
    presetId: "cc-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["claude_code"],
    platform: "anthropic",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/v1",
    defaultModel: "claude-3-5-sonnet-20241022",
    notes: "【强烈推荐】AI Singularity 官方高速全球网络，免翻墙调用各大模型。",
  },

  // ── OpenAI Codex ────────────────────────────────────────────────────────
  {
    presetId: "codex-openai-official",
    name: "OpenAI (官方直连)",
    category: "official",
    toolTargets: ["codex"],
    platform: "open_ai",
    icon: "openai",
    iconColor: "#00A67E",
    websiteUrl: "https://chatgpt.com/codex",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    defaultBaseUrl: "https://api.openai.com/v1",
    defaultModel: "gpt-5.4",
    notes: "OpenAI Codex 官方直连，需要境外网络。",
  },
  {
    presetId: "codex-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/v1",
    defaultModel: "gpt-5.4",
    notes: "AI Singularity 官方高速中转，支持 OpenAI Codex 协议。",
  },

  // ── Gemini CLI ──────────────────────────────────────────────────────────
  {
    presetId: "gemini-google-official",
    name: "Google (官方直连)",
    category: "official",
    toolTargets: ["gemini_cli"],
    platform: "gemini",
    icon: "gemini",
    iconColor: "#4285F4",
    websiteUrl: "https://ai.google.dev/",
    apiKeyUrl: "https://aistudio.google.com/app/apikey",
    defaultBaseUrl: "https://generativelanguage.googleapis.com",
    defaultModel: "gemini-2.5-pro",
    notes: "直连 Google 官方 Gemini，需特定网络环境。",
  },
  {
    presetId: "gemini-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["gemini_cli"],
    platform: "custom",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/gemini",
    defaultModel: "gemini-2.5-pro",
    notes: "AI Singularity 高速中转，无需境外网络。",
  },

  // ── OpenCode ────────────────────────────────────────────────────────────
  {
    presetId: "opencode-openai-official",
    name: "OpenAI (官方直连)",
    category: "official",
    toolTargets: ["open_code"],
    platform: "open_ai",
    icon: "openai",
    iconColor: "#00A67E",
    websiteUrl: "https://platform.openai.com",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    defaultBaseUrl: "https://api.openai.com/v1",
    defaultModel: "gpt-5.4",
    notes: "OpenAI 官方直连，需境外网络。",
  },
  {
    presetId: "opencode-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/v1",
    defaultModel: "gpt-5.4",
    notes: "AI Singularity 高速中转，兼容 OpenAI 协议。",
  },

  // ── OpenClaw ────────────────────────────────────────────────────────────
  {
    presetId: "openclaw-openai-official",
    name: "OpenAI (官方直连)",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "open_ai",
    icon: "openai",
    iconColor: "#00A67E",
    websiteUrl: "https://platform.openai.com",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    defaultBaseUrl: "https://api.openai.com/v1",
    defaultModel: "gpt-4o",
    notes: "OpenAI 官方直连，需境外网络。",
  },
  {
    presetId: "openclaw-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/v1",
    defaultModel: "gpt-4o",
    notes: "AI Singularity 高速中转，兼容 OpenAI 协议。",
  },

  // ── Aider ───────────────────────────────────────────────────────────────
  {
    presetId: "aider-anthropic-official",
    name: "Anthropic (官方直连)",
    category: "official",
    toolTargets: ["aider"],
    platform: "anthropic",
    icon: "anthropic",
    iconColor: "#D97757",
    websiteUrl: "https://www.anthropic.com",
    apiKeyUrl: "https://console.anthropic.com/settings/keys",
    defaultBaseUrl: "https://api.anthropic.com",
    defaultModel: "claude-opus-4-5",
    notes: "直连 Anthropic 官方 API，需境外网络。",
  },
  {
    presetId: "aider-singularity-relay",
    name: "AI Singularity 网络 (推荐)",
    category: "relay",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "singularity",
    iconColor: "#8B5CF6",
    websiteUrl: "https://aisingularity.com",
    apiKeyUrl: "https://aisingularity.com/dashboard/tokens",
    defaultBaseUrl: "https://api.aisingularity.com/v1",
    defaultModel: "claude-3-5-sonnet-20241022",
    notes: "AI Singularity 高速中转，支持 Aider 所有主流模型。",
  },

  // ── Claude Code — 第三方平台 ─────────────────────────────────────────────
  {
    presetId: "cc-deepseek",
    name: "DeepSeek",
    category: "official",
    toolTargets: ["claude_code"],
    platform: "custom",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    websiteUrl: "https://www.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    notes: "DeepSeek 官方 API，兼容 OpenAI 协议，国内直连无需翻墙，价格极具竞争力。",
  },
  {
    presetId: "cc-moonshot",
    name: "Moonshot (Kimi)",
    category: "official",
    toolTargets: ["claude_code"],
    platform: "custom",
    icon: "moonshot",
    iconColor: "#1A1A2E",
    websiteUrl: "https://www.moonshot.cn",
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    defaultModel: "moonshot-v1-8k",
    notes: "月之暗面 Kimi 官方 API，兼容 OpenAI 协议，国内直连，支持超长上下文。",
  },
  {
    presetId: "cc-siliconflow",
    name: "SiliconFlow",
    category: "official",
    toolTargets: ["claude_code"],
    platform: "custom",
    icon: "siliconflow",
    iconColor: "#0EA5E9",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    notes: "硅基流动，聚合多家开源模型，兼容 OpenAI 协议，国内直连，按量计费。",
  },
  {
    presetId: "cc-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["claude_code"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "anthropic/claude-3.5-sonnet",
    notes: "OpenRouter 聚合路由，支持 Claude / GPT / Gemini 等数百个模型，统一 OpenAI 协议。",
  },

  // ── Codex — 第三方平台 ───────────────────────────────────────────────────
  {
    presetId: "codex-deepseek",
    name: "DeepSeek",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    websiteUrl: "https://www.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    notes: "DeepSeek 官方 API，兼容 OpenAI 协议，国内直连，代码能力出色。",
  },
  {
    presetId: "codex-moonshot",
    name: "Moonshot (Kimi)",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "moonshot",
    iconColor: "#1A1A2E",
    websiteUrl: "https://www.moonshot.cn",
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    defaultModel: "moonshot-v1-8k",
    notes: "月之暗面 Kimi 官方 API，兼容 OpenAI 协议，国内直连，长文本处理能力强。",
  },
  {
    presetId: "codex-siliconflow",
    name: "SiliconFlow",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "siliconflow",
    iconColor: "#0EA5E9",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    notes: "硅基流动，聚合多家开源模型，兼容 OpenAI 协议，国内直连。",
  },
  {
    presetId: "codex-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "anthropic/claude-3.5-sonnet",
    notes: "OpenRouter 聚合路由，支持数百个模型，统一 OpenAI 协议接入。",
  },
  {
    presetId: "codex-aliyun",
    name: "阿里云百炼",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "aliyun",
    iconColor: "#FF6A00",
    websiteUrl: "https://bailian.console.aliyun.com",
    apiKeyUrl: "https://bailian.console.aliyun.com/#/api-key",
    defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    defaultModel: "qwen-max",
    notes: "阿里云百炼平台，通义千问系列模型，兼容 OpenAI 协议，国内直连，企业级稳定性。",
  },
  {
    presetId: "codex-bytedance",
    name: "字节豆包",
    category: "official",
    toolTargets: ["codex"],
    platform: "custom",
    icon: "bytedance",
    iconColor: "#1664FF",
    websiteUrl: "https://www.volcengine.com/product/ark",
    apiKeyUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    defaultBaseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    defaultModel: "doubao-pro-32k",
    notes: "字节跳动火山引擎豆包大模型，兼容 OpenAI 协议，国内直连，性价比高。",
  },

  // ── Gemini CLI — 第三方平台 ──────────────────────────────────────────────
  {
    presetId: "gemini-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["gemini_cli"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "google/gemini-2.5-pro",
    notes: "通过 OpenRouter 访问 Gemini 模型，无需境外网络，支持 OpenAI 兼容协议。",
  },

  // ── OpenCode — 第三方平台 ────────────────────────────────────────────────
  {
    presetId: "opencode-deepseek",
    name: "DeepSeek",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    websiteUrl: "https://www.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    notes: "DeepSeek 官方 API，兼容 OpenAI 协议，国内直连，代码能力出色。",
  },
  {
    presetId: "opencode-moonshot",
    name: "Moonshot (Kimi)",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "moonshot",
    iconColor: "#1A1A2E",
    websiteUrl: "https://www.moonshot.cn",
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    defaultModel: "moonshot-v1-8k",
    notes: "月之暗面 Kimi 官方 API，兼容 OpenAI 协议，国内直连，长文本处理能力强。",
  },
  {
    presetId: "opencode-siliconflow",
    name: "SiliconFlow",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "siliconflow",
    iconColor: "#0EA5E9",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    notes: "硅基流动，聚合多家开源模型，兼容 OpenAI 协议，国内直连，按量计费。",
  },
  {
    presetId: "opencode-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "anthropic/claude-3.5-sonnet",
    notes: "OpenRouter 聚合路由，支持数百个模型，统一 OpenAI 协议接入。",
  },
  {
    presetId: "opencode-aliyun",
    name: "阿里云百炼",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "aliyun",
    iconColor: "#FF6A00",
    websiteUrl: "https://bailian.console.aliyun.com",
    apiKeyUrl: "https://bailian.console.aliyun.com/#/api-key",
    defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    defaultModel: "qwen-max",
    notes: "阿里云百炼平台，通义千问系列模型，兼容 OpenAI 协议，国内直连，企业级稳定性。",
  },
  {
    presetId: "opencode-bytedance",
    name: "字节豆包",
    category: "official",
    toolTargets: ["open_code"],
    platform: "custom",
    icon: "bytedance",
    iconColor: "#1664FF",
    websiteUrl: "https://www.volcengine.com/product/ark",
    apiKeyUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    defaultBaseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    defaultModel: "doubao-pro-32k",
    notes: "字节跳动火山引擎豆包大模型，兼容 OpenAI 协议，国内直连，性价比高。",
  },

  // ── OpenClaw — 第三方平台 ────────────────────────────────────────────────
  {
    presetId: "openclaw-deepseek",
    name: "DeepSeek",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    websiteUrl: "https://www.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    notes: "DeepSeek 官方 API，兼容 OpenAI 协议，国内直连，价格极具竞争力。",
  },
  {
    presetId: "openclaw-moonshot",
    name: "Moonshot (Kimi)",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "moonshot",
    iconColor: "#1A1A2E",
    websiteUrl: "https://www.moonshot.cn",
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    defaultModel: "moonshot-v1-8k",
    notes: "月之暗面 Kimi 官方 API，兼容 OpenAI 协议，国内直连，支持超长上下文。",
  },
  {
    presetId: "openclaw-siliconflow",
    name: "SiliconFlow",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "siliconflow",
    iconColor: "#0EA5E9",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    notes: "硅基流动，聚合多家开源模型，兼容 OpenAI 协议，国内直连，按量计费。",
  },
  {
    presetId: "openclaw-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "anthropic/claude-3.5-sonnet",
    notes: "OpenRouter 聚合路由，支持数百个模型，统一 OpenAI 协议接入。",
  },
  {
    presetId: "openclaw-aliyun",
    name: "阿里云百炼",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "aliyun",
    iconColor: "#FF6A00",
    websiteUrl: "https://bailian.console.aliyun.com",
    apiKeyUrl: "https://bailian.console.aliyun.com/#/api-key",
    defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    defaultModel: "qwen-max",
    notes: "阿里云百炼平台，通义千问系列模型，兼容 OpenAI 协议，国内直连，企业级稳定性。",
  },
  {
    presetId: "openclaw-bytedance",
    name: "字节豆包",
    category: "official",
    toolTargets: ["open_claw"],
    platform: "custom",
    icon: "bytedance",
    iconColor: "#1664FF",
    websiteUrl: "https://www.volcengine.com/product/ark",
    apiKeyUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    defaultBaseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    defaultModel: "doubao-pro-32k",
    notes: "字节跳动火山引擎豆包大模型，兼容 OpenAI 协议，国内直连，性价比高。",
  },

  // ── Aider — 第三方平台 ───────────────────────────────────────────────────
  {
    presetId: "aider-deepseek",
    name: "DeepSeek",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "deepseek",
    iconColor: "#4D6BFE",
    websiteUrl: "https://www.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    notes: "DeepSeek 官方 API，兼容 OpenAI 协议，国内直连，代码能力出色，Aider 官方推荐。",
  },
  {
    presetId: "aider-moonshot",
    name: "Moonshot (Kimi)",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "moonshot",
    iconColor: "#1A1A2E",
    websiteUrl: "https://www.moonshot.cn",
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    defaultModel: "moonshot-v1-8k",
    notes: "月之暗面 Kimi 官方 API，兼容 OpenAI 协议，国内直连，长文本处理能力强。",
  },
  {
    presetId: "aider-siliconflow",
    name: "SiliconFlow",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "siliconflow",
    iconColor: "#0EA5E9",
    websiteUrl: "https://siliconflow.cn",
    apiKeyUrl: "https://cloud.siliconflow.cn/account/ak",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    notes: "硅基流动，聚合多家开源模型，兼容 OpenAI 协议，国内直连，按量计费。",
  },
  {
    presetId: "aider-openrouter",
    name: "OpenRouter",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "openrouter",
    iconColor: "#6366F1",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "anthropic/claude-3.5-sonnet",
    notes: "OpenRouter 聚合路由，支持数百个模型，Aider 可通过 --openai-api-base 指定。",
  },
  {
    presetId: "aider-aliyun",
    name: "阿里云百炼",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "aliyun",
    iconColor: "#FF6A00",
    websiteUrl: "https://bailian.console.aliyun.com",
    apiKeyUrl: "https://bailian.console.aliyun.com/#/api-key",
    defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    defaultModel: "qwen-max",
    notes: "阿里云百炼平台，通义千问系列模型，兼容 OpenAI 协议，国内直连，企业级稳定性。",
  },
  {
    presetId: "aider-bytedance",
    name: "字节豆包",
    category: "official",
    toolTargets: ["aider"],
    platform: "custom",
    icon: "bytedance",
    iconColor: "#1664FF",
    websiteUrl: "https://www.volcengine.com/product/ark",
    apiKeyUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    defaultBaseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    defaultModel: "doubao-pro-32k",
    notes: "字节跳动火山引擎豆包大模型，兼容 OpenAI 协议，国内直连，性价比高。",
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// 工具函数
// ─────────────────────────────────────────────────────────────────────────────

/** 按照当前激活的终端标签过滤预设 */
export function filterPresetsByTool(tool?: ToolTarget): ProviderPreset[] {
  if (!tool) return PROVIDER_PRESETS;
  return PROVIDER_PRESETS.filter(
    (p) => !p.toolTargets || p.toolTargets.length === 0 || p.toolTargets.includes(tool),
  );
}

/** 按分类分组 */
export function groupPresetsByCategory(): Record<ProviderCategory, ProviderPreset[]> {
  const result: Record<ProviderCategory, ProviderPreset[]> = {
    relay: [], official: [], custom: [],
  };
  for (const preset of PROVIDER_PRESETS) {
    result[preset.category].push(preset);
  }
  return result;
}

/** 按照终端 + 分类分组 */
export function groupPresetsByToolAndCategory(
  tool?: ToolTarget,
): Record<ProviderCategory, ProviderPreset[]> {
  const base = filterPresetsByTool(tool);
  const result: Record<ProviderCategory, ProviderPreset[]> = {
    relay: [], official: [], custom: [],
  };
  for (const preset of base) {
    result[preset.category].push(preset);
  }
  return result;
}

/** 搜索预设 */
export function searchPresets(query: string, tool?: ToolTarget): ProviderPreset[] {
  const q = query.toLowerCase();
  return filterPresetsByTool(tool).filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.presetId.toLowerCase().includes(q) ||
      (p.defaultBaseUrl?.toLowerCase().includes(q) ?? false),
  );
}

/** 分类显示名映射 */
export const CATEGORY_LABELS: Record<ProviderCategory, string> = {
  relay:    "👑 AI Singularity 高速中转",
  official: "🌐 官方原厂直连",
  custom:   "⚙️ 自定义节点",
};
