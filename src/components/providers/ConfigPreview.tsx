/**
 * ConfigPreview — 渠道配置文件格式实时预览
 *
 * 根据当前表单的 base_url、api_key、tool_targets 和 ToolSpecificConfigs，
 * 实时计算并展示将要写入各工具配置文件的内容。
 *
 * 参考 cc-switch UniversalProviderFormModal 中的 claudeConfigJson / codexConfigJson / geminiConfigJson useMemo 实现。
 */

import { useMemo, useState } from "react";
import type { ProviderConfig, ToolTarget, ToolSpecificConfigs } from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH } from "../../types";
import "./ConfigPreview.css";

// ─────────────────────────────────────────────────────────────────────────────
// 配置内容生成函数
// ─────────────────────────────────────────────────────────────────────────────

type ProviderExtraConfig = {
  apiKeyField?: string;
  envInjection?: string;
  projectId?: string;
};

function parseProviderExtraConfig(provider?: ProviderConfig): ProviderExtraConfig {
  try {
    if (!provider?.extra_config) return {};
    const parsed = JSON.parse(provider.extra_config);
    return parsed && typeof parsed === "object" ? parsed as ProviderExtraConfig : {};
  } catch {
    return {};
  }
}

function resolveToolModel(
  cfg: { model?: string; model_name?: string } | undefined,
  provider?: ProviderConfig,
): string {
  const model = cfg?.model || cfg?.model_name || provider?.model_name;
  return model?.trim() ? model : "(使用 Provider 默认模型)";
}

function normalizeCodexBaseUrl(baseUrl: string): string {
  if (!baseUrl) return "https://api.example.com/v1";
  return baseUrl.endsWith("/v1") ? baseUrl : `${baseUrl.replace(/\/+$/, "")}/v1`;
}

function escapeEnvValue(value: string): string {
  if (!value) return "\"\"";
  return /[\s#"\\]/.test(value)
    ? `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`
    : value;
}

function resolveGeminiEnvMode(provider?: ProviderConfig): "standard" | "legacy" {
  const mode = parseProviderExtraConfig(provider).envInjection;
  return mode === "legacy" ? "legacy" : "standard";
}

function resolvePreviewPath(tool: ToolTarget): string {
  if (tool === "codex") return "~/.codex/config.toml + ~/.codex/auth.json";
  if (tool === "gemini_cli") return "~/.gemini/settings.json + ~/.gemini/.env";
  return TOOL_TARGET_CONFIG_PATH[tool];
}

function buildClaudePreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["claude_code"],
  provider?: ProviderConfig,
): string {
  const extra = parseProviderExtraConfig(provider);
  const authField = extra.apiKeyField === "ANTHROPIC_AUTH_TOKEN"
    ? "ANTHROPIC_AUTH_TOKEN"
    : "ANTHROPIC_API_KEY";
  const model = resolveToolModel(cfg, provider);
  const env: Record<string, string> = {};

  if (apiKey) env[authField] = apiKey;
  if (baseUrl) env.ANTHROPIC_BASE_URL = baseUrl;
  env.ANTHROPIC_MODEL = model;
  if (cfg?.reasoningModel) env.ANTHROPIC_REASONING_MODEL = cfg.reasoningModel;
  if (cfg?.haikuModel) env.ANTHROPIC_DEFAULT_HAIKU_MODEL = cfg.haikuModel;
  if (cfg?.sonnetModel) env.ANTHROPIC_DEFAULT_SONNET_MODEL = cfg.sonnetModel;
  if (cfg?.opusModel) env.ANTHROPIC_DEFAULT_OPUS_MODEL = cfg.opusModel;

  return JSON.stringify({ env }, null, 2);
}

function buildCodexPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["codex"],
  provider?: ProviderConfig,
): string {
  const model = resolveToolModel(cfg, provider);
  const effort = cfg?.reasoningEffort || "high";
  const codexUrl = normalizeCodexBaseUrl(baseUrl);
  const providerName = provider?.name || "Custom Provider";

  return `model_provider = "ai_singularity"
model = "${model}"
model_reasoning_effort = "${effort}"
disable_response_storage = true

[model_providers.ai_singularity]
name = "${providerName}"
base_url = "${codexUrl}"
wire_api = "responses"
requires_openai_auth = true

# ~/.codex/auth.json
# OPENAI_API_KEY = "${apiKey ? apiKey.slice(0, 8) + "..." : "your-api-key"}"`;
}

function buildGeminiPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["gemini_cli"],
  provider?: ProviderConfig,
): string {
  const extra = parseProviderExtraConfig(provider);
  const settings: Record<string, unknown> = {
    security: {
      auth: {
        selectedType: "gemini-api-key",
      },
    },
  };
  const model = resolveToolModel(cfg, provider);
  const envLines: string[] = [];
  const envMode = resolveGeminiEnvMode(provider);

  if (baseUrl) settings.GOOGLE_GEMINI_BASE_URL = baseUrl;
  settings.model = model;
  if (extra.projectId?.trim()) settings.projectId = extra.projectId.trim();
  if (apiKey) {
    envLines.push(`GEMINI_API_KEY=${escapeEnvValue(apiKey)}`);
    if (envMode === "legacy") {
      envLines.push(`GOOGLE_API_KEY=${escapeEnvValue(apiKey)}`);
    }
  }
  if (baseUrl) envLines.push(`GOOGLE_GEMINI_BASE_URL=${escapeEnvValue(baseUrl)}`);
  if (extra.projectId?.trim()) {
    envLines.push(`GOOGLE_CLOUD_PROJECT=${escapeEnvValue(extra.projectId.trim())}`);
  }
  envLines.push(`GEMINI_MODEL=${escapeEnvValue(model)}`);

  return [
    "# ~/.gemini/settings.json",
    JSON.stringify(settings, null, 2),
    "",
    "# ~/.gemini/.env",
    ...(envLines.length > 0 ? envLines : ["# (当前不会写入额外环境变量)"]),
  ].join("\n");
}

function buildOpenCodePreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["open_code"],
  provider?: ProviderConfig,
): string {
  const model = resolveToolModel(cfg, provider);
  const providerKey = provider?.id?.replace(/-/g, "_") || "custom";
  return JSON.stringify({
    providers: {
      [providerKey]: {
        npm: "@ai-sdk/openai-compatible",
        options: {
          baseURL: baseUrl || "https://api.example.com/v1",
          apiKey: apiKey || "{env:OPENAI_API_KEY}",
        },
        models: {
          [model]: {
            name: model,
          },
        },
      },
    },
    mcpServers: {},
  }, null, 2);
}

function buildOpenClawPreview(
  baseUrl: string,
  _apiKey: string,
  cfg: ToolSpecificConfigs["open_claw"],
  provider?: ProviderConfig,
): string {
  const model = resolveToolModel(cfg, provider);
  return JSON.stringify({
    openai_base_url: baseUrl || "https://api.example.com/v1",
    model,
  }, null, 2);
}

function buildAiderPreview(
  baseUrl: string,
  _apiKey: string,
  cfg: ToolSpecificConfigs["aider"],
  provider?: ProviderConfig,
): string {
  const model = resolveToolModel(cfg, provider);
  const lines: string[] = ["# ~/.aider.conf.yml"];
  lines.push(`model: ${model}`);
  if (baseUrl) lines.push(`openai-api-base: ${baseUrl}`);
  return lines.join("\n");
}

// ─────────────────────────────────────────────────────────────────────────────
// PreviewBlock 子组件
// ─────────────────────────────────────────────────────────────────────────────
function PreviewBlock({
  tool,
  content,
  lang,
}: {
  tool: ToolTarget;
  content: string;
  lang: "json" | "toml" | "yaml";
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(content).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const LANG_LABELS: Record<string, string> = { json: "JSON", toml: "TOML", yaml: "YAML" };

  return (
    <div className="cfg-preview-block">
      <div className="cfg-preview-header">
        <div className="cfg-preview-meta">
          <span className="cfg-preview-tool">{TOOL_TARGET_LABELS[tool]}</span>
          <span className="cfg-preview-path">{resolvePreviewPath(tool)}</span>
          <span className="cfg-preview-lang-badge">{LANG_LABELS[lang]}</span>
        </div>
        <button className="cfg-copy-btn" onClick={handleCopy}>
          {copied ? "✓ 已复制" : "复制"}
        </button>
      </div>
      <pre className="cfg-preview-code">{content}</pre>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 主组件
// ─────────────────────────────────────────────────────────────────────────────
interface ConfigPreviewProps {
  baseUrl: string;
  apiKey: string;
  toolTargets: ToolTarget[];
  toolConfigs: ToolSpecificConfigs;
  provider?: ProviderConfig;
}

export default function ConfigPreview({
  baseUrl,
  apiKey,
  toolTargets,
  toolConfigs,
  provider,
}: ConfigPreviewProps) {
  const previews = useMemo(() => {
    const result: Array<{ tool: ToolTarget; content: string; lang: "json" | "toml" | "yaml" }> = [];

    if (toolTargets.includes("claude_code")) {
      result.push({
        tool: "claude_code",
        content: buildClaudePreview(baseUrl, apiKey, toolConfigs.claude_code, provider),
        lang: "json",
      });
    }
    if (toolTargets.includes("codex")) {
      result.push({
        tool: "codex",
        content: buildCodexPreview(baseUrl, apiKey, toolConfigs.codex, provider),
        lang: "toml",
      });
    }
    if (toolTargets.includes("gemini_cli")) {
      result.push({
        tool: "gemini_cli",
        content: buildGeminiPreview(baseUrl, apiKey, toolConfigs.gemini_cli, provider),
        lang: "yaml",
      });
    }
    if (toolTargets.includes("open_code")) {
      result.push({
        tool: "open_code",
        content: buildOpenCodePreview(baseUrl, apiKey, toolConfigs.open_code, provider),
        lang: "json",
      });
    }
    if (toolTargets.includes("open_claw")) {
      result.push({
        tool: "open_claw",
        content: buildOpenClawPreview(baseUrl, apiKey, toolConfigs.open_claw, provider),
        lang: "json",
      });
    }
    if (toolTargets.includes("aider")) {
      result.push({
        tool: "aider",
        content: buildAiderPreview(baseUrl, apiKey, toolConfigs.aider, provider),
        lang: "yaml",
      });
    }

    return result;
  }, [baseUrl, apiKey, provider, toolTargets, toolConfigs]);

  if (previews.length === 0) return null;

  return (
    <div className="cfg-preview-container">

      <div className="cfg-preview-list">
        {previews.map(({ tool, content, lang }) => (
          <PreviewBlock key={tool} tool={tool} content={content} lang={lang} />
        ))}
      </div>
    </div>
  );
}
