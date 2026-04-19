import type { Dispatch, SetStateAction } from "react";
import type { ToolTarget } from "../../types";
import { ProviderAdvancedConfig, type ProviderExtraConfig } from "./ProviderAdvancedConfig";
import { JsonConfigEditor } from "./JsonConfigEditor";
import { AnthropicFormFields } from "./forms/AnthropicFormFields";
import { OpenAIFormFields } from "./forms/OpenAIFormFields";
import { GeminiFormFields } from "./forms/GeminiFormFields";
import type { ProviderFormState } from "./providerModalFormTypes";

function updateClaudeOverride(
  advancedCfg: ProviderExtraConfig,
  setForm: Dispatch<SetStateAction<ProviderFormState>>,
  field: "haikuModel" | "sonnetModel" | "opusModel" | "reasoningModel",
  value: string
) {
  const toolConfigs = advancedCfg.tool_configs || {};
  const claudeConfig = toolConfigs.claude_code || {};
  const extra = {
    ...advancedCfg,
    tool_configs: {
      ...toolConfigs,
      claude_code: {
        ...claudeConfig,
        [field]: value || undefined,
      },
    },
  };
  setForm((current) => ({ ...current, extra_config: JSON.stringify(extra, null, 2) }));
}

function ProviderPlatformOverrides({
  advancedCfg,
  form,
  setForm,
}: {
  advancedCfg: ProviderExtraConfig;
  form: ProviderFormState;
  setForm: Dispatch<SetStateAction<ProviderFormState>>;
}) {
  const wrapperStyle = {
    padding: "0 16px",
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
    borderRadius: 12,
    marginTop: 12,
    paddingBottom: 16,
  } as const;

  if (form.platform === "anthropic") {
    return (
      <div style={wrapperStyle}>
        <AnthropicFormFields
          value={advancedCfg}
          onChange={(cfg) => setForm((current) => ({ ...current, extra_config: JSON.stringify(cfg, null, 2) }))}
        />
      </div>
    );
  }

  if (form.platform === "open_ai") {
    return (
      <div style={wrapperStyle}>
        <OpenAIFormFields
          value={advancedCfg}
          onChange={(cfg) => setForm((current) => ({ ...current, extra_config: JSON.stringify(cfg, null, 2) }))}
        />
      </div>
    );
  }

  if (form.platform === "gemini") {
    return (
      <div style={wrapperStyle}>
        <GeminiFormFields
          value={advancedCfg}
          onChange={(cfg) => setForm((current) => ({ ...current, extra_config: JSON.stringify(cfg, null, 2) }))}
        />
      </div>
    );
  }

  if (form.platform === "custom") {
    return (
      <div style={wrapperStyle}>
        <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>
          底层配置重写 (Advanced)
        </div>
        <JsonConfigEditor
          value={form.extra_config}
          onChange={(value) => setForm((current) => ({ ...current, extra_config: value }))}
        />
      </div>
    );
  }

  return null;
}

export function ProviderModalAdvancedTab({
  advancedCfg,
  fixedTool,
  form,
  setForm,
}: {
  advancedCfg: ProviderExtraConfig;
  fixedTool?: ToolTarget;
  form: ProviderFormState;
  setForm: Dispatch<SetStateAction<ProviderFormState>>;
}) {
  const updateNumberField = (field: "temperature" | "maxTokens", value: string, parser: (next: string) => number) => {
    const extra = { ...advancedCfg };
    if (value === "") {
      delete extra[field];
    } else {
      extra[field] = parser(value);
    }
    setForm((current) => ({ ...current, extra_config: JSON.stringify(extra, null, 2) }));
  };

  const showClaudeOverrides = fixedTool === "claude_code" || form.tool_targets.includes("claude_code");

  return (
    <>
      <div className="form-grid-2" style={{ marginBottom: 16 }}>
        <div className="form-row">
          <label className="form-label">模型温度 (Temperature)</label>
          <input
            type="number"
            step="0.1"
            min="0"
            max="2"
            className="form-input font-mono"
            placeholder="平台默认"
            value={advancedCfg.temperature ?? ""}
            onChange={(event) => updateNumberField("temperature", event.target.value, parseFloat)}
          />
        </div>
        <div className="form-row">
          <label className="form-label">最大输出 (Max Tokens)</label>
          <input
            type="number"
            step="1"
            min="1"
            className="form-input font-mono"
            placeholder="平台默认"
            value={advancedCfg.maxTokens ?? ""}
            onChange={(event) => updateNumberField("maxTokens", event.target.value, (next) => parseInt(next, 10))}
          />
        </div>
      </div>

      {showClaudeOverrides && (
        <div
          style={{
            marginBottom: 16,
            padding: "16px",
            background: "var(--color-surface-raised)",
            borderRadius: 12,
            border: "1px solid var(--color-border)",
          }}
        >
          <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>
            Claude 大小杯模型映射 (Overrides)
          </div>

          <div className="form-grid-2">
            <div className="form-row">
              <label className="form-label">Haiku 小模型</label>
              <input
                className="form-input font-mono"
                placeholder="claude-3-5-haiku-20241022"
                value={advancedCfg.tool_configs?.claude_code?.haikuModel || ""}
                onChange={(event) => updateClaudeOverride(advancedCfg, setForm, "haikuModel", event.target.value)}
              />
            </div>
            <div className="form-row">
              <label className="form-label">Sonnet 中模型</label>
              <input
                className="form-input font-mono"
                placeholder="claude-3-5-sonnet-20241022"
                value={advancedCfg.tool_configs?.claude_code?.sonnetModel || ""}
                onChange={(event) => updateClaudeOverride(advancedCfg, setForm, "sonnetModel", event.target.value)}
              />
            </div>
          </div>

          <div className="form-grid-2">
            <div className="form-row">
              <label className="form-label">Opus 大模型</label>
              <input
                className="form-input font-mono"
                placeholder="claude-3-opus-20240229"
                value={advancedCfg.tool_configs?.claude_code?.opusModel || ""}
                onChange={(event) => updateClaudeOverride(advancedCfg, setForm, "opusModel", event.target.value)}
              />
            </div>
            <div className="form-row">
              <label className="form-label">Reasoning 推理模型</label>
              <input
                className="form-input font-mono"
                placeholder="claude-3-7-sonnet-20250219"
                value={advancedCfg.tool_configs?.claude_code?.reasoningModel || ""}
                onChange={(event) => updateClaudeOverride(advancedCfg, setForm, "reasoningModel", event.target.value)}
              />
            </div>
          </div>
          <div className="form-hint" style={{ marginTop: 2 }}>
            当对应场景触发时，终端将使用以上设定替代官方硬编码来向本节点发送请求。
          </div>
        </div>
      )}

      <div style={{ marginTop: 8, marginBottom: 16 }}>
        <ProviderAdvancedConfig
          value={advancedCfg}
          onChange={(cfg) => setForm((current) => ({ ...current, extra_config: JSON.stringify(cfg, null, 2) }))}
        />
        <ProviderPlatformOverrides advancedCfg={advancedCfg} form={form} setForm={setForm} />
      </div>
    </>
  );
}
