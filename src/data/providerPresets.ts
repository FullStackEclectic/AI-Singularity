/**
 * AI Singularity — Provider 预设库
 *
 * 参考 cc-switch 的 provider presets 数据（重新设计结构，工具无关）
 * 每条 preset 描述一个 API 供应商，由 SyncService 自动转换为各工具格式
 */

export type ProviderCategory =
  | 'official'        // 官方（Anthropic/OpenAI/Google）
  | 'cn_official'     // 国内大模型官方
  | 'cloud_provider'  // 云厂商（AWS/Azure/NVIDIA）
  | 'aggregator'      // 聚合平台（OpenRouter/SiliconFlow）
  | 'third_party'     // 第三方中继
  | 'custom'          // 用户自定义

/** 模板变量定义（用于动态填入 URL 占位符） */
export interface TemplateVar {
  key: string
  label: string
  placeholder: string
  defaultValue?: string
}

export interface ProviderPreset {
  /** 预设唯一标识（用于去重） */
  presetId: string
  /** 显示名称 */
  name: string
  /** 分类 */
  category: ProviderCategory
  /** 目标平台（对应后端的 Platform 结构，默认 Custom） */
  platform?: 'OpenAI' | 'Anthropic' | 'Gemini' | 'Custom' | 'Ollama' | 'AzureOpenAI' | 'AWSBedrock'
  /** 图标标识（对应 BrandIcon 组件） */
  icon?: string
  /** 图标颜色（Hex） */
  iconColor?: string
  /** 官网链接 */
  websiteUrl: string
  /** API Key 申请链接（若与官网不同） */
  apiKeyUrl?: string
  /** 默认 Base URL（可含 ${VAR} 占位符） */
  defaultBaseUrl?: string
  /** 默认模型名 */
  defaultModel?: string
  /** 模板变量列表（需用户填入） */
  templateVars?: TemplateVar[]
  /** 备用 endpoint 候选（用于测速） */
  endpointCandidates?: string[]
  /** 是否需要 OAuth（如 GitHub Copilot） */
  requiresOAuth?: boolean
  /** 备注提示（显示在表单底部） */
  notes?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// 预设数据
// ─────────────────────────────────────────────────────────────────────────────

export const PROVIDER_PRESETS: ProviderPreset[] = [

  // ── 官方 ──────────────────────────────────────────────────────────────────

  {
    presetId: 'anthropic-official',
    name: 'Anthropic (官方)',
    category: 'official',
    platform: 'Anthropic',
    icon: 'anthropic',
    iconColor: '#D97757',
    websiteUrl: 'https://www.anthropic.com/claude-code',
    apiKeyUrl: 'https://console.anthropic.com/settings/keys',
    defaultBaseUrl: undefined, // 使用默认，不注入 ANTHROPIC_BASE_URL
    defaultModel: 'claude-opus-4-5',
    notes: '直连 Anthropic 官方 API，需要境外网络',
  },
  {
    presetId: 'openai-official',
    name: 'OpenAI (官方)',
    category: 'official',
    platform: 'OpenAI',
    icon: 'openai',
    iconColor: '#000000',
    websiteUrl: 'https://platform.openai.com',
    apiKeyUrl: 'https://platform.openai.com/api-keys',
    defaultBaseUrl: 'https://api.openai.com/v1',
    defaultModel: 'gpt-4o',
  },
  {
    presetId: 'google-gemini-official',
    name: 'Google Gemini (官方)',
    category: 'official',
    platform: 'Gemini',
    icon: 'gemini',
    iconColor: '#4285F4',
    websiteUrl: 'https://aistudio.google.com',
    apiKeyUrl: 'https://aistudio.google.com/app/apikey',
    defaultBaseUrl: 'https://generativelanguage.googleapis.com',
    defaultModel: 'gemini-2.5-pro',
  },

  // ── 国内大模型官方 ─────────────────────────────────────────────────────────

  {
    presetId: 'deepseek',
    name: 'DeepSeek',
    category: 'cn_official',
    icon: 'deepseek',
    iconColor: '#1E88E5',
    websiteUrl: 'https://platform.deepseek.com',
    apiKeyUrl: 'https://platform.deepseek.com/api_keys',
    defaultBaseUrl: 'https://api.deepseek.com/anthropic',
    defaultModel: 'DeepSeek-V3.2',
  },
  {
    presetId: 'zhipu-glm-cn',
    name: '智谱 GLM (国内)',
    category: 'cn_official',
    icon: 'zhipu',
    iconColor: '#0F62FE',
    websiteUrl: 'https://open.bigmodel.cn',
    apiKeyUrl: 'https://www.bigmodel.cn/claude-code',
    defaultBaseUrl: 'https://open.bigmodel.cn/api/anthropic',
    defaultModel: 'glm-5',
  },
  {
    presetId: 'zhipu-zai',
    name: 'Z.ai (智谱国际)',
    category: 'cn_official',
    icon: 'zhipu',
    iconColor: '#0F62FE',
    websiteUrl: 'https://z.ai',
    apiKeyUrl: 'https://z.ai/subscribe',
    defaultBaseUrl: 'https://api.z.ai/api/anthropic',
    defaultModel: 'glm-5',
  },
  {
    presetId: 'kimi-moonshot',
    name: 'Kimi (Moonshot)',
    category: 'cn_official',
    icon: 'kimi',
    iconColor: '#6366F1',
    websiteUrl: 'https://platform.moonshot.cn/console',
    apiKeyUrl: 'https://platform.moonshot.cn/console/api-keys',
    defaultBaseUrl: 'https://api.moonshot.cn/anthropic',
    defaultModel: 'kimi-k2.5',
  },
  {
    presetId: 'kimi-for-coding',
    name: 'Kimi For Coding',
    category: 'cn_official',
    icon: 'kimi',
    iconColor: '#6366F1',
    websiteUrl: 'https://www.kimi.com/coding/docs/',
    defaultBaseUrl: 'https://api.kimi.com/coding/',
    defaultModel: 'kimi-k2.5',
    notes: 'Kimi 专为编程设计的 API 端点',
  },
  {
    presetId: 'minimax-cn',
    name: 'MiniMax (国内)',
    category: 'cn_official',
    icon: 'minimax',
    iconColor: '#FF6B6B',
    websiteUrl: 'https://platform.minimaxi.com',
    apiKeyUrl: 'https://platform.minimaxi.com/subscribe/coding-plan',
    defaultBaseUrl: 'https://api.minimaxi.com/anthropic',
    defaultModel: 'MiniMax-M2.7',
  },
  {
    presetId: 'minimax-global',
    name: 'MiniMax (国际)',
    category: 'cn_official',
    icon: 'minimax',
    iconColor: '#FF6B6B',
    websiteUrl: 'https://platform.minimax.io',
    apiKeyUrl: 'https://platform.minimax.io/subscribe/coding-plan',
    defaultBaseUrl: 'https://api.minimax.io/anthropic',
    defaultModel: 'MiniMax-M2.7',
  },
  {
    presetId: 'stepfun',
    name: 'StepFun (阶跃星辰)',
    category: 'cn_official',
    icon: 'stepfun',
    iconColor: '#005AFF',
    websiteUrl: 'https://platform.stepfun.ai',
    apiKeyUrl: 'https://platform.stepfun.ai/interface-key',
    defaultBaseUrl: 'https://api.stepfun.ai/v1',
    defaultModel: 'step-3.5-flash',
  },
  {
    presetId: 'doubao-seed',
    name: '豆包 DouBaoSeed',
    category: 'cn_official',
    icon: 'doubao',
    iconColor: '#3370FF',
    websiteUrl: 'https://www.volcengine.com/product/doubao',
    defaultBaseUrl: 'https://ark.cn-beijing.volces.com/api/coding',
    defaultModel: 'doubao-seed-2-0-code-preview-latest',
  },
  {
    presetId: 'bailian-aliyun',
    name: '阿里云百炼',
    category: 'cn_official',
    icon: 'aliyun',
    iconColor: '#FF6A00',
    websiteUrl: 'https://bailian.console.aliyun.com',
    defaultBaseUrl: 'https://dashscope.aliyuncs.com/apps/anthropic',
  },
  {
    presetId: 'bailian-for-coding',
    name: '阿里云百炼 For Coding',
    category: 'cn_official',
    icon: 'aliyun',
    iconColor: '#FF6A00',
    websiteUrl: 'https://bailian.console.aliyun.com',
    defaultBaseUrl: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
    notes: '专为编程场景优化的百炼端点',
  },
  {
    presetId: 'modelscope',
    name: 'ModelScope (魔搭)',
    category: 'cn_official',
    icon: 'modelscope',
    iconColor: '#624AFF',
    websiteUrl: 'https://modelscope.cn',
    defaultBaseUrl: 'https://api-inference.modelscope.cn',
    defaultModel: 'ZhipuAI/GLM-5',
  },
  {
    presetId: 'xiaomi-mimo',
    name: '小米 MiMo',
    category: 'cn_official',
    icon: 'xiaomi',
    iconColor: '#FF6900',
    websiteUrl: 'https://platform.xiaomimimo.com',
    apiKeyUrl: 'https://platform.xiaomimimo.com/#/console/api-keys',
    defaultBaseUrl: 'https://api.xiaomimimo.com/anthropic',
    defaultModel: 'mimo-v2-pro',
  },

  // ── 云厂商 ─────────────────────────────────────────────────────────────────

  {
    presetId: 'aws-bedrock-aksk',
    name: 'AWS Bedrock (AKSK)',
    category: 'cloud_provider',
    platform: 'AWSBedrock',
    icon: 'aws',
    iconColor: '#FF9900',
    websiteUrl: 'https://aws.amazon.com/bedrock/',
    defaultBaseUrl: 'https://bedrock-runtime.${AWS_REGION}.amazonaws.com',
    defaultModel: 'global.anthropic.claude-opus-4-6-v1',
    templateVars: [
      { key: 'AWS_REGION', label: 'AWS Region', placeholder: 'us-west-2', defaultValue: 'us-west-2' },
      { key: 'AWS_ACCESS_KEY_ID', label: 'Access Key ID', placeholder: 'AKIA...' },
      { key: 'AWS_SECRET_ACCESS_KEY', label: 'Secret Access Key', placeholder: 'your-secret-key' },
    ],
    notes: 'AKSK 认证方式，需要在 extra_config 中存储 AWS 凭证',
  },
  {
    presetId: 'aws-bedrock-apikey',
    name: 'AWS Bedrock (API Key)',
    category: 'cloud_provider',
    platform: 'AWSBedrock',
    icon: 'aws',
    iconColor: '#FF9900',
    websiteUrl: 'https://aws.amazon.com/bedrock/',
    defaultBaseUrl: 'https://bedrock-runtime.${AWS_REGION}.amazonaws.com',
    defaultModel: 'global.anthropic.claude-opus-4-6-v1',
    templateVars: [
      { key: 'AWS_REGION', label: 'AWS Region', placeholder: 'us-west-2', defaultValue: 'us-west-2' },
    ],
  },
  {
    presetId: 'azure-openai',
    name: 'Azure OpenAI',
    category: 'cloud_provider',
    platform: 'AzureOpenAI',
    icon: 'azure',
    iconColor: '#0078D4',
    websiteUrl: 'https://azure.microsoft.com/products/ai-services/openai-service',
    defaultBaseUrl: 'https://${AZURE_RESOURCE}.openai.azure.com/openai/deployments/${DEPLOYMENT_ID}',
    templateVars: [
      { key: 'AZURE_RESOURCE', label: 'Resource Name', placeholder: 'my-resource' },
      { key: 'DEPLOYMENT_ID', label: 'Deployment ID', placeholder: 'gpt-4o' },
    ],
  },
  {
    presetId: 'nvidia-nim',
    name: 'NVIDIA NIM',
    category: 'cloud_provider',
    icon: 'nvidia',
    iconColor: '#76B900',
    websiteUrl: 'https://build.nvidia.com',
    apiKeyUrl: 'https://build.nvidia.com/settings/api-keys',
    defaultBaseUrl: 'https://integrate.api.nvidia.com',
    defaultModel: 'moonshotai/kimi-k2.5',
  },

  // ── 聚合平台 ───────────────────────────────────────────────────────────────

  {
    presetId: 'openrouter',
    name: 'OpenRouter',
    category: 'aggregator',
    icon: 'openrouter',
    iconColor: '#6566F1',
    websiteUrl: 'https://openrouter.ai',
    apiKeyUrl: 'https://openrouter.ai/keys',
    defaultBaseUrl: 'https://openrouter.ai/api',
    defaultModel: 'anthropic/claude-sonnet-4-6',
    endpointCandidates: ['https://openrouter.ai/api'],
  },
  {
    presetId: 'siliconflow-cn',
    name: 'SiliconFlow (国内)',
    category: 'aggregator',
    icon: 'siliconflow',
    iconColor: '#6E29F6',
    websiteUrl: 'https://siliconflow.cn',
    apiKeyUrl: 'https://cloud.siliconflow.cn',
    defaultBaseUrl: 'https://api.siliconflow.cn',
    defaultModel: 'Pro/MiniMaxAI/MiniMax-M2.7',
  },
  {
    presetId: 'siliconflow-global',
    name: 'SiliconFlow (国际)',
    category: 'aggregator',
    icon: 'siliconflow',
    iconColor: '#6E29F6',
    websiteUrl: 'https://siliconflow.com',
    defaultBaseUrl: 'https://api.siliconflow.com',
    defaultModel: 'MiniMaxAI/MiniMax-M2.7',
  },
  {
    presetId: 'aihubmix',
    name: 'AiHubMix',
    category: 'aggregator',
    icon: 'aihubmix',
    iconColor: '#006FFB',
    websiteUrl: 'https://aihubmix.com',
    defaultBaseUrl: 'https://aihubmix.com',
    endpointCandidates: ['https://aihubmix.com', 'https://api.aihubmix.com'],
  },
  {
    presetId: 'novita-ai',
    name: 'Novita AI',
    category: 'aggregator',
    icon: 'novita',
    iconColor: '#000000',
    websiteUrl: 'https://novita.ai',
    defaultBaseUrl: 'https://api.novita.ai/anthropic',
    defaultModel: 'zai-org/glm-5',
  },

  // ── GitHub Copilot（OAuth） ────────────────────────────────────────────────

  {
    presetId: 'github-copilot',
    name: 'GitHub Copilot',
    category: 'third_party',
    icon: 'github',
    iconColor: '#181717',
    websiteUrl: 'https://github.com/features/copilot',
    defaultBaseUrl: 'https://api.githubcopilot.com',
    defaultModel: 'claude-opus-4-6',
    requiresOAuth: true,
    notes: '需要 GitHub Copilot 订阅。使用 OAuth 授权，无需 API Key（Phase 2 实装）。',
  },
]

// ─────────────────────────────────────────────────────────────────────────────
// 工具函数
// ─────────────────────────────────────────────────────────────────────────────

/** 按分类分组 */
export function groupPresetsByCategory(): Record<ProviderCategory, ProviderPreset[]> {
  const result: Record<ProviderCategory, ProviderPreset[]> = {
    official: [],
    cn_official: [],
    cloud_provider: [],
    aggregator: [],
    third_party: [],
    custom: [],
  }
  for (const preset of PROVIDER_PRESETS) {
    result[preset.category].push(preset)
  }
  return result
}

/** 搜索预设 */
export function searchPresets(query: string): ProviderPreset[] {
  const q = query.toLowerCase()
  return PROVIDER_PRESETS.filter(
    p =>
      p.name.toLowerCase().includes(q) ||
      p.presetId.toLowerCase().includes(q) ||
      (p.defaultBaseUrl?.toLowerCase().includes(q) ?? false),
  )
}

/** 分类显示名映射 */
export const CATEGORY_LABELS: Record<ProviderCategory, string> = {
  official: '官方',
  cn_official: '国内大模型',
  cloud_provider: '云厂商',
  aggregator: '聚合平台',
  third_party: '第三方中继',
  custom: '自定义',
}
