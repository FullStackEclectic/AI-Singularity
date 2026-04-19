import type { Dispatch, SetStateAction } from "react";
import type { Platform, ToolTarget } from "../../types";
import { PLATFORM_LABELS, TOOL_TARGET_LABELS } from "../../types";
import { ALL_TOOLS, TOOL_ICONS } from "./providerModalShared";
import type { ProviderFormState } from "./providerModalFormTypes";

export function ProviderModalBasicTab({
  fixedTool,
  form,
  isEditing,
  isFetchingModels,
  modelFetchError,
  modelOptions,
  onFetchModels,
  setForm,
  toggleTool,
}: {
  fixedTool?: ToolTarget;
  form: ProviderFormState;
  isEditing: boolean;
  isFetchingModels: boolean;
  modelFetchError: string;
  modelOptions: string[];
  onFetchModels: () => Promise<void>;
  setForm: Dispatch<SetStateAction<ProviderFormState>>;
  toggleTool: (tool: ToolTarget) => void;
}) {
  return (
    <>
      <div className="form-grid-2" style={{ marginBottom: 16 }}>
        <div>
          <label className="form-label">配置名称 *</label>
          <input
            className="form-input"
            placeholder="例如：DeepSeek 官方"
            value={form.name}
            onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
          />
        </div>
        <div>
          <label className="form-label">平台类型</label>
          <select
            className="form-input"
            value={form.platform}
            onChange={(event) => setForm((current) => ({ ...current, platform: event.target.value as Platform }))}
          >
            {(Object.entries(PLATFORM_LABELS) as [Platform, string][]).map(([key, label]) => (
              <option key={key} value={key}>
                {label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="form-row">
        <label className="form-label">Base URL（接口地址）</label>
        <input
          className="form-input font-mono"
          placeholder="https://api.example.com/anthropic"
          value={form.base_url}
          onChange={(event) => setForm((current) => ({ ...current, base_url: event.target.value }))}
        />
        <p className="form-hint" style={{ marginTop: 4 }}>
          留空使用平台默认地址（Anthropic 官方无需填写）
        </p>
      </div>

      <div className="form-row">
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 6,
          }}
        >
          <label className="form-label" style={{ marginBottom: 0 }}>
            API Key 凭证
          </label>
          {form.api_key_url && (
            <a
              href={form.api_key_url}
              target="_blank"
              rel="noreferrer"
              className="text-accent"
              style={{ fontSize: 12, textDecoration: "none", fontWeight: 500 }}
            >
              获取专属 Key ↗
            </a>
          )}
        </div>
        <input
          type="password"
          className="form-input font-mono"
          placeholder={isEditing ? "(留空保持原凭证不变)" : "sk-..."}
          value={form.api_key_value}
          onChange={(event) => setForm((current) => ({ ...current, api_key_value: event.target.value }))}
        />
        <p className="form-hint" style={{ marginTop: 4 }}>提交后将被加密保存至凭证中心</p>
      </div>

      <div className="form-row">
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 6,
          }}
        >
          <label className="form-label" style={{ marginBottom: 0 }}>
            默认模型
          </label>
          <button
            type="button"
            className="btn btn-ghost btn-xs"
            onClick={() => void onFetchModels()}
            disabled={isFetchingModels}
            title="从当前 Provider 拉取模型列表"
          >
            {isFetchingModels ? "获取中..." : "获取模型"}
          </button>
        </div>
        <input
          className="form-input font-mono"
          placeholder="claude-opus-4-5"
          value={form.model_name}
          onChange={(event) => setForm((current) => ({ ...current, model_name: event.target.value }))}
        />
        {modelOptions.length > 0 && (
          <select
            className="form-input"
            value={form.model_name}
            onChange={(event) => setForm((current) => ({ ...current, model_name: event.target.value }))}
            style={{ marginTop: 8 }}
          >
            {modelOptions.map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </select>
        )}
        {modelFetchError && (
          <div className="form-hint" style={{ color: "var(--color-danger)", marginTop: 6 }}>
            {modelFetchError}
          </div>
        )}
      </div>

      <div className="form-row" style={{ marginTop: 12 }}>
        <label className="form-label">备注（可选）</label>
        <textarea
          className="form-input"
          rows={2}
          placeholder="备用、测试或者特定项目的专属账单节点？"
          value={form.notes}
          onChange={(event) => setForm((current) => ({ ...current, notes: event.target.value }))}
          style={{ resize: "vertical" }}
        />
      </div>

      {!fixedTool && (
        <div style={{ marginTop: 24, marginBottom: 16 }}>
          <div className="form-section-title">同步到哪些工具 *</div>
          <p className="form-hint" style={{ marginTop: -8, marginBottom: 10 }}>
            激活此 Provider 时，将自动写入以下工具的配置文件
          </p>
          <div className="tool-targets-grid">
            {ALL_TOOLS.map((tool) => (
              <label
                key={tool}
                className={`tool-target-chip ${form.tool_targets.includes(tool) ? "checked" : ""}`}
              >
                <input
                  type="checkbox"
                  checked={form.tool_targets.includes(tool)}
                  onChange={() => toggleTool(tool)}
                  style={{ display: "none" }}
                />
                <span className="tool-chip-icon">{TOOL_ICONS[tool]}</span>
                {TOOL_TARGET_LABELS[tool]}
              </label>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
