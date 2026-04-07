/**
 * ConfigPreview — 渠道配置文件格式实时预览
 *
 * 根据当前表单的 base_url、api_key、tool_targets 和 ToolSpecificConfigs，
 * 实时计算并展示将要写入各工具配置文件的内容。
 *
 * 参考 cc-switch UniversalProviderFormModal 中的 claudeConfigJson / codexConfigJson / geminiConfigJson useMemo 实现。
 */

import { useMemo, useState } from "react";
import type { ToolTarget, ToolSpecificConfigs } from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH } from "../../types";

// ─────────────────────────────────────────────────────────────────────────────
// 配置内容生成函数
// ─────────────────────────────────────────────────────────────────────────────

function buildClaudePreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["claude_code"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  const haiku  = cfg?.haikuModel  || undefined;
  const sonnet = cfg?.sonnetModel || undefined;
  const opus   = cfg?.opusModel   || undefined;

  const env: Record<string, string> = {
    ANTHROPIC_AUTH_TOKEN: apiKey || "sk-...",
  };
  if (baseUrl) env["ANTHROPIC_BASE_URL"] = baseUrl;
  env["ANTHROPIC_MODEL"] = model;
  if (haiku)  env["ANTHROPIC_DEFAULT_HAIKU_MODEL"]  = haiku;
  if (sonnet) env["ANTHROPIC_DEFAULT_SONNET_MODEL"] = sonnet;
  if (opus)   env["ANTHROPIC_DEFAULT_OPUS_MODEL"]   = opus;

  return JSON.stringify({ env }, null, 2);
}

function buildCodexPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["codex"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  const effort = cfg?.reasoningEffort || "high";
  const codexUrl = baseUrl
    ? (baseUrl.endsWith("/v1") ? baseUrl : `${baseUrl.replace(/\/+$/, "")}/v1`)
    : "https://api.example.com/v1";

  return `model_provider = "custom"
model = "${model}"
model_reasoning_effort = "${effort}"
disable_response_storage = true

[model_providers.custom]
name = "Custom Provider"
base_url = "${codexUrl}"
wire_api = "responses"
requires_openai_auth = true

# auth (配置环境变量)
# OPENAI_API_KEY = "${apiKey ? apiKey.slice(0, 8) + "..." : "your-api-key"}"`;
}

function buildGeminiPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["gemini_cli"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  const env: Record<string, string> = {
    GEMINI_API_KEY: apiKey || "your-api-key",
  };
  if (baseUrl) env["GOOGLE_GEMINI_BASE_URL"] = baseUrl;
  env["GEMINI_MODEL"] = model;

  return JSON.stringify({ env }, null, 2);
}

function buildOpenCodePreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["open_code"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  return JSON.stringify({
    providers: {
      custom: {
        api_key:  apiKey || "your-api-key",
        base_url: baseUrl || "https://api.example.com",
        model,
      },
    },
  }, null, 2);
}

function buildOpenClawPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["open_claw"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  return JSON.stringify({
    provider: {
      base_url: baseUrl || "https://api.example.com",
      api_key:  apiKey || "your-api-key",
      model,
    },
  }, null, 2);
}

function buildAiderPreview(
  baseUrl: string,
  apiKey: string,
  cfg: ToolSpecificConfigs["aider"],
): string {
  const model = cfg?.model || "(使用 Provider 默认模型)";
  const lines: string[] = ["# ~/.aider.conf.yml"];
  lines.push(`model: anthropic/${model}`);
  if (baseUrl) lines.push(`anthropic-api-base: ${baseUrl}`);
  lines.push(`# 环境变量: ANTHROPIC_API_KEY=${apiKey ? apiKey.slice(0, 8) + "..." : "your-api-key"}`);
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
          <span className="cfg-preview-path">{TOOL_TARGET_CONFIG_PATH[tool]}</span>
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
}

export default function ConfigPreview({
  baseUrl,
  apiKey,
  toolTargets,
  toolConfigs,
}: ConfigPreviewProps) {
  const previews = useMemo(() => {
    const result: Array<{ tool: ToolTarget; content: string; lang: "json" | "toml" | "yaml" }> = [];

    if (toolTargets.includes("claude_code")) {
      result.push({
        tool: "claude_code",
        content: buildClaudePreview(baseUrl, apiKey, toolConfigs.claude_code),
        lang: "json",
      });
    }
    if (toolTargets.includes("codex")) {
      result.push({
        tool: "codex",
        content: buildCodexPreview(baseUrl, apiKey, toolConfigs.codex),
        lang: "toml",
      });
    }
    if (toolTargets.includes("gemini_cli")) {
      result.push({
        tool: "gemini_cli",
        content: buildGeminiPreview(baseUrl, apiKey, toolConfigs.gemini_cli),
        lang: "json",
      });
    }
    if (toolTargets.includes("open_code")) {
      result.push({
        tool: "open_code",
        content: buildOpenCodePreview(baseUrl, apiKey, toolConfigs.open_code),
        lang: "json",
      });
    }
    if (toolTargets.includes("open_claw")) {
      result.push({
        tool: "open_claw",
        content: buildOpenClawPreview(baseUrl, apiKey, toolConfigs.open_claw),
        lang: "json",
      });
    }
    if (toolTargets.includes("aider")) {
      result.push({
        tool: "aider",
        content: buildAiderPreview(baseUrl, apiKey, toolConfigs.aider),
        lang: "yaml",
      });
    }

    return result;
  }, [baseUrl, apiKey, toolTargets, toolConfigs]);

  if (previews.length === 0) return null;

  return (
    <div className="cfg-preview-container">
      <div className="form-section-title" style={{ marginTop: 0 }}>
        配置文件预览
        <span className="cfg-preview-hint">以下内容将在激活时写入对应配置文件</span>
      </div>
      <div className="cfg-preview-list">
        {previews.map(({ tool, content, lang }) => (
          <PreviewBlock key={tool} tool={tool} content={content} lang={lang} />
        ))}
      </div>
    </div>
  );
}
