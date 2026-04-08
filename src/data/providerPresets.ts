/**
 * AI Singularity — Provider 预设库
 *
 * 每个终端只保留两条预设：
 *   1. 官方原厂直连
 *   2. AI Singularity 高速中转（推荐）
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
