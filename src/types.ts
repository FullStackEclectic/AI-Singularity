export type Platform =
  | "open_ai"
  | "anthropic"
  | "gemini"
  | "deep_seek"
  | "aliyun"
  | "bytedance"
  | "moonshot"
  | "zhipu"
  | "aws_bedrock"
  | "nvidia_nim"
  | "custom";

export type KeyStatus = "unknown" | "valid" | "invalid" | "expired" | "banned" | "rate_limit";

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
}

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

export const PLATFORM_LABELS: Record<Platform, string> = {
  open_ai: "OpenAI",
  anthropic: "Anthropic",
  gemini: "Google Gemini",
  deep_seek: "DeepSeek",
  aliyun: "阿里云百炼",
  bytedance: "字节豆包",
  moonshot: "Moonshot (Kimi)",
  zhipu: "智谱 GLM",
  aws_bedrock: "AWS Bedrock",
  nvidia_nim: "NVIDIA NIM",
  custom: "自定义接口",
};

export const STATUS_LABELS: Record<KeyStatus, string> = {
  unknown: "未检测",
  valid: "正常",
  invalid: "无效",
  expired: "已过期",
  banned: "被封禁",
  rate_limit: "限速中",
};
