/**
 * ToolConfigPanel — 渠道专属配置面板
 *
 * 当某个 ToolTarget 被选中时，动态展开对应渠道的配置输入项。
 * 每个渠道的配置字段各不相同（参考 cc-switch UniversalProviderFormModal 设计）。
 */

import type {
  ToolTarget,
  ToolSpecificConfigs,
  ClaudeToolConfig,
  CodexToolConfig,
  GeminiToolConfig,
} from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH } from "../../types";
import "./ToolConfigPanel.css";

const TOOL_ICONS: Record<ToolTarget, string> = {
  claude_code: "🤖",
  codex:       "🧠",
  gemini_cli:  "✨",
  open_code:   "🌐",
  open_claw:   "🦞",
  aider:       "💻",
};

// ─────────────────────────────────────────────────────────────────────────────
// 单个字段输入
// ─────────────────────────────────────────────────────────────────────────────
function ConfigField({
  label,
  value,
  placeholder,
  onChange,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="tool-cfg-field">
      <label className="tool-cfg-field-label">{label}</label>
      <input
        className="form-input tool-cfg-input"
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Claude Code 配置
// ─────────────────────────────────────────────────────────────────────────────
function ClaudeConfigPanel({
  config,
  onChange,
}: {
  config: ClaudeToolConfig;
  onChange: (c: ClaudeToolConfig) => void;
}) {
  const upd = (k: keyof ClaudeToolConfig) => (v: string) =>
    onChange({ ...config, [k]: v });

  return (
    <div className="tool-cfg-fields">
      <ConfigField
        label="主模型"
        value={config.model ?? ""}
        placeholder="claude-sonnet-4-5（留空使用 Provider 默认模型）"
        onChange={upd("model")}
      />
      <div className="tool-cfg-subfields">
        <ConfigField
          label="Haiku 轻量模型"
          value={config.haikuModel ?? ""}
          placeholder="claude-haiku-4-5"
          onChange={upd("haikuModel")}
        />
        <ConfigField
          label="Sonnet 标准模型"
          value={config.sonnetModel ?? ""}
          placeholder="claude-sonnet-4-5"
          onChange={upd("sonnetModel")}
        />
        <ConfigField
          label="Opus 旗舰模型"
          value={config.opusModel ?? ""}
          placeholder="claude-opus-4-5"
          onChange={upd("opusModel")}
        />
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Codex 配置
// ─────────────────────────────────────────────────────────────────────────────
function CodexConfigPanel({
  config,
  onChange,
}: {
  config: CodexToolConfig;
  onChange: (c: CodexToolConfig) => void;
}) {
  const upd = (k: keyof CodexToolConfig) => (v: string) =>
    onChange({ ...config, [k]: v });

  return (
    <div className="tool-cfg-fields">
      <div className="tool-cfg-subfields">
        <ConfigField
          label="模型"
          value={config.model ?? ""}
          placeholder="gpt-4o（留空使用 Provider 默认模型）"
          onChange={upd("model")}
        />
        <div className="tool-cfg-field">
          <label className="tool-cfg-field-label">Reasoning Effort</label>
          <select
            className="form-input tool-cfg-input"
            value={config.reasoningEffort ?? "high"}
            onChange={(e) => upd("reasoningEffort")(e.target.value)}
          >
            <option value="high">high — 深度推理</option>
            <option value="medium">medium — 均衡</option>
            <option value="low">low — 快速响应</option>
          </select>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Gemini CLI 配置
// ─────────────────────────────────────────────────────────────────────────────
function GeminiConfigPanel({
  config,
  onChange,
}: {
  config: GeminiToolConfig;
  onChange: (c: GeminiToolConfig) => void;
}) {
  return (
    <div className="tool-cfg-fields">
      <ConfigField
        label="模型"
        value={config.model ?? ""}
        placeholder="gemini-2.5-pro（留空使用 Provider 默认模型）"
        onChange={(v) => onChange({ ...config, model: v })}
      />
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 通用单模型字段（OpenCode / OpenClaw / Aider）
// ─────────────────────────────────────────────────────────────────────────────
function SimpleModelPanel({
  config,
  onChange,
  placeholder,
}: {
  config: { model?: string };
  onChange: (c: { model?: string }) => void;
  placeholder?: string;
}) {
  return (
    <div className="tool-cfg-fields">
      <ConfigField
        label="模型"
        value={config.model ?? ""}
        placeholder={placeholder ?? "留空使用 Provider 默认模型"}
        onChange={(v) => onChange({ ...config, model: v })}
      />
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 主组件
// ─────────────────────────────────────────────────────────────────────────────
interface ToolConfigPanelProps {
  tool: ToolTarget;
  configs: ToolSpecificConfigs;
  onUpdate: (tool: ToolTarget, config: ToolSpecificConfigs[ToolTarget]) => void;
}

export default function ToolConfigPanel({ tool, configs, onUpdate }: ToolConfigPanelProps) {
  const updater =
    <T extends ToolSpecificConfigs[ToolTarget]>(c: T) =>
      onUpdate(tool, c);

  return (
    <div className="tool-config-panel">
      <div className="tool-config-panel-header">
        <span className="tcp-icon">{TOOL_ICONS[tool]}</span>
        <div>
          <span className="tcp-name">{TOOL_TARGET_LABELS[tool]}</span>
          <span className="tcp-path">{TOOL_TARGET_CONFIG_PATH[tool]}</span>
        </div>
        <span className="tcp-badge">渠道专属配置</span>
      </div>

      <div className="tool-config-panel-body">
        {tool === "claude_code" && (
          <ClaudeConfigPanel
            config={configs.claude_code ?? {}}
            onChange={updater}
          />
        )}
        {tool === "codex" && (
          <CodexConfigPanel
            config={configs.codex ?? {}}
            onChange={updater}
          />
        )}
        {tool === "gemini_cli" && (
          <GeminiConfigPanel
            config={configs.gemini_cli ?? {}}
            onChange={updater}
          />
        )}
        {tool === "open_code" && (
          <SimpleModelPanel
            config={configs.open_code ?? {}}
            onChange={updater}
            placeholder="claude-sonnet-4-5"
          />
        )}
        {tool === "open_claw" && (
          <SimpleModelPanel
            config={configs.open_claw ?? {}}
            onChange={updater}
            placeholder="claude-opus-4-5"
          />
        )}
        {tool === "aider" && (
          <SimpleModelPanel
            config={configs.aider ?? {}}
            onChange={updater}
            placeholder="claude-sonnet-4-5"
          />
        )}
      </div>
    </div>
  );
}
