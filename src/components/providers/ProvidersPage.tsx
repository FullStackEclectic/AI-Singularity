import { useEffect, useState, useMemo } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, ToolTarget, Platform } from "../../types";
import {
  TOOL_TARGET_LABELS,
  TOOL_TARGET_CONFIG_PATH,
  PLATFORM_LABELS,
  PROVIDER_CATEGORY_LABELS,
  parseToolTargets,
} from "../../types";
import {
  PROVIDER_PRESETS,
  groupPresetsByCategory,
  CATEGORY_LABELS,
  type ProviderPreset,
  type ProviderCategory,
} from "../../data/providerPresets";
import "./ProvidersPage.css";

const ALL_TOOLS: ToolTarget[] = ["claude_code", "codex", "gemini_cli", "open_code", "open_claw", "aider"];

// ── 工具图标映射 ─────────────────────────────────────────────────────────────

const TOOL_ICONS: Record<ToolTarget, string> = {
  claude_code: "🤖",
  codex:       "🧠",
  gemini_cli:  "✨",
  open_code:   "🌐",
  open_claw:   "🦞",
  aider:       "💻",
};

// ─────────────────────────────────────────────────────────────────────────────
// Main Page
// ─────────────────────────────────────────────────────────────────────────────

export default function ProvidersPage() {
  const { providers, isLoading, error, fetch, switchProvider, deleteProvider } = useProviderStore();
  const [showAdd, setShowAdd] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig | null>(null);

  useEffect(() => { fetch(); }, [fetch]);

  const handleSwitch = async (id: string) => {
    await switchProvider(id);
  };

  const handleDelete = async (id: string) => {
    if (!confirm("确定删除此 Provider？删除后相关工具配置将使用下一个可用的 Provider。")) return;
    await deleteProvider(id);
  };

  return (
    <div className="providers-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">AI 工具接入层</h1>
          <p className="page-subtitle">
            一条配置，同时同步到多个 AI 编程工具（Claude Code / Codex / Gemini CLI 等）
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
          <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
            ＋ 添加 Provider
          </button>
        </div>
      </div>

      {error && <div className="form-error" style={{ marginBottom: 16 }}>⚠ {error}</div>}

      <div className="providers-body">
        {isLoading && providers.length === 0 ? (
          <div className="empty-state">
            <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
            <span>加载中...</span>
          </div>
        ) : providers.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">⚡</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>暂无 Provider 配置</h3>
            <p>点击「添加 Provider」，从预设库快速接入大模型服务</p>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              ＋ 添加第一个 Provider
            </button>
          </div>
        ) : (
          <div className="providers-grid">
            {providers.map((p) => (
              <ProviderCard
                key={p.id}
                provider={p}
                onActivate={() => handleSwitch(p.id)}
                onEdit={() => setEditingProvider(p)}
                onDelete={() => handleDelete(p.id)}
              />
            ))}
          </div>
        )}
      </div>

      {showAdd && (
        <ProviderModal
          onClose={() => setShowAdd(false)}
          onSuccess={() => { setShowAdd(false); fetch(); }}
        />
      )}
      {editingProvider && (
        <ProviderModal
          initialProvider={editingProvider}
          onClose={() => setEditingProvider(null)}
          onSuccess={() => { setEditingProvider(null); fetch(); }}
        />
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Card
// ─────────────────────────────────────────────────────────────────────────────

function ProviderCard({
  provider,
  onActivate,
  onEdit,
  onDelete,
}: {
  provider: ProviderConfig;
  onActivate: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const targets = parseToolTargets(provider);

  return (
    <div className={`card provider-card ${provider.is_active ? "active-card" : ""} animate-fade-in`}>
      {/* Header */}
      <div className="provider-header">
        <div className="provider-title">
          <div
            className="provider-icon-badge"
            style={{ background: provider.icon_color ? `${provider.icon_color}22` : "var(--color-surface-raised)" }}
          >
            <span style={{ fontSize: 20 }}>
              {provider.platform === "anthropic" ? "🟠" :
               provider.platform === "open_ai"   ? "🟢" :
               provider.platform === "gemini"     ? "🔵" :
               provider.platform === "deep_seek"  ? "🔷" :
               provider.platform === "open_router"? "🟣" : "⚙️"}
            </span>
          </div>
          <div>
            <h3 style={{ margin: 0, fontSize: 15 }}>{provider.name}</h3>
            <div className="text-muted" style={{ fontSize: 12, marginTop: 2 }}>
              {PLATFORM_LABELS[provider.platform] ?? provider.platform}
            </div>
          </div>
        </div>
        <div className="provider-card-actions">
          <button
            className={`btn-activate ${provider.is_active ? "active" : ""}`}
            onClick={onActivate}
            title={provider.is_active ? "当前激活" : "点击激活"}
          >
            {provider.is_active ? "✓ 激活中" : "激活"}
          </button>
        </div>
      </div>

      {/* Details */}
      <div className="provider-details">
        {provider.model_name && (
          <div className="detail-row">
            <span className="text-muted">默认模型</span>
            <span className="font-mono" style={{ fontSize: 12 }}>{provider.model_name}</span>
          </div>
        )}
        {provider.base_url && (
          <div className="detail-row">
            <span className="text-muted">接口地址</span>
            <span className="font-mono truncate-text" style={{ fontSize: 11 }} title={provider.base_url}>
              {provider.base_url}
            </span>
          </div>
        )}
      </div>

      {/* Tool targets */}
      <div className="provider-targets">
        <span className="text-muted" style={{ fontSize: 11 }}>同步到：</span>
        <div className="targets-chips">
          {ALL_TOOLS.map((t) => (
            <span
              key={t}
              className={`target-chip ${targets.includes(t) ? "active-chip" : "inactive-chip"}`}
              title={TOOL_TARGET_CONFIG_PATH[t]}
            >
              {TOOL_ICONS[t]} {TOOL_TARGET_LABELS[t]}
            </span>
          ))}
        </div>
      </div>

      {/* Footer actions */}
      <div className="provider-card-footer">
        {provider.website_url && (
          <a href={provider.website_url} target="_blank" rel="noreferrer" className="btn btn-ghost btn-xs">
            官网 ↗
          </a>
        )}
        <button className="btn btn-ghost btn-xs" onClick={onEdit}>编辑</button>
        <button className="btn btn-danger-ghost btn-xs" onClick={onDelete}>删除</button>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Modal（新增 + 编辑）
// ─────────────────────────────────────────────────────────────────────────────

interface ProviderFormState {
  name: string;
  platform: Platform;
  base_url: string;
  model_name: string;
  api_key_value: string;  // 明文 key（提交后加密存储）
  tool_targets: ToolTarget[];
  website_url: string;
  api_key_url: string;
  notes: string;
}

function ProviderModal({
  initialProvider,
  onClose,
  onSuccess,
}: {
  initialProvider?: ProviderConfig;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { add, update } = useProviderStore();

  // Preset panel
  const [step, setStep] = useState<"preset" | "form">(initialProvider ? "form" : "preset");
  const [presetSearch, setPresetSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<ProviderCategory | "all">("all");

  const isEditing = !!initialProvider;

  const [form, setForm] = useState<ProviderFormState>(() => {
    if (initialProvider) {
      return {
        name:          initialProvider.name,
        platform:      initialProvider.platform,
        base_url:      initialProvider.base_url ?? "",
        model_name:    initialProvider.model_name,
        api_key_value: "",
        tool_targets:  parseToolTargets(initialProvider),
        website_url:   initialProvider.website_url ?? "",
        api_key_url:   initialProvider.api_key_url ?? "",
        notes:         initialProvider.notes ?? "",
      };
    }
    return {
      name: "", platform: "custom", base_url: "", model_name: "",
      api_key_value: "", tool_targets: ["claude_code"],
      website_url: "", api_key_url: "", notes: "",
    };
  });

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");

  // ── 预设过滤 ──────────────────────────────────────────────────────────────

  const groupedPresets = useMemo(() => groupPresetsByCategory(), []);
  const filteredPresets = useMemo(() => {
    const base = selectedCategory === "all"
      ? PROVIDER_PRESETS
      : groupedPresets[selectedCategory] ?? [];
    if (!presetSearch.trim()) return base;
    const q = presetSearch.toLowerCase();
    return base.filter(p =>
      p.name.toLowerCase().includes(q) ||
      (p.defaultBaseUrl?.toLowerCase().includes(q) ?? false),
    );
  }, [selectedCategory, presetSearch, groupedPresets]);

  const applyPreset = (preset: ProviderPreset) => {
    setForm(f => ({
      ...f,
      name:       preset.name,
      platform:   presetIdToPlatform(preset.presetId),
      base_url:   preset.defaultBaseUrl ?? "",
      model_name: preset.defaultModel ?? "",
      website_url: preset.websiteUrl ?? "",
      api_key_url: preset.apiKeyUrl ?? "",
      notes:      preset.notes ?? "",
    }));
    setStep("form");
  };

  // ── 表单提交 ──────────────────────────────────────────────────────────────

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) { setError("名称不能为空"); return; }
    if (form.tool_targets.length === 0) { setError("请至少选择一个同步目标"); return; }

    setIsSubmitting(true);
    setError("");
    try {
      const payload: any = {
        id:           initialProvider?.id ?? "",
        name:         form.name.trim(),
        platform:     form.platform,
        base_url:     form.base_url.trim() || null,
        model_name:   form.model_name.trim(),
        is_active:    initialProvider?.is_active ?? false,
        tool_targets: JSON.stringify(form.tool_targets),
        website_url:  form.website_url.trim() || null,
        api_key_url:  form.api_key_url.trim() || null,
        notes:        form.notes.trim() || null,
        created_at:   initialProvider?.created_at ?? "",
        updated_at:   "",
      };
      if (isEditing) {
        await update(payload);
      } else {
        await add(payload);
      }
      onSuccess();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  const toggleTool = (tool: ToolTarget) => {
    setForm(f => ({
      ...f,
      tool_targets: f.tool_targets.includes(tool)
        ? f.tool_targets.filter(t => t !== tool)
        : [...f.tool_targets, tool],
    }));
  };

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{isEditing ? "编辑 Provider" : (step === "preset" ? "选择预设模板" : "新增 Provider")}</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        {/* Step 1: 预设库 */}
        {step === "preset" && !isEditing && (
          <div className="modal-body preset-panel">
            <div className="preset-toolbar">
              <input
                className="form-input"
                placeholder="搜索供应商…"
                value={presetSearch}
                onChange={e => setPresetSearch(e.target.value)}
                style={{ flex: 1 }}
              />
              <select
                className="form-input"
                value={selectedCategory}
                onChange={e => setSelectedCategory(e.target.value as any)}
                style={{ width: 140 }}
              >
                <option value="all">全部分类</option>
                {(Object.keys(CATEGORY_LABELS) as ProviderCategory[]).map(c => (
                  <option key={c} value={c}>{CATEGORY_LABELS[c]}</option>
                ))}
              </select>
            </div>

            <div className="preset-grid">
              {filteredPresets.map(preset => (
                <button
                  key={preset.presetId}
                  className="preset-card"
                  onClick={() => applyPreset(preset)}
                >
                  <div className="preset-name">{preset.name}</div>
                  <div className="preset-meta text-muted">{CATEGORY_LABELS[preset.category]}</div>
                  {preset.defaultBaseUrl && (
                    <div className="preset-url font-mono">{preset.defaultBaseUrl}</div>
                  )}
                </button>
              ))}
            </div>

            <div className="modal-footer">
              <button className="btn btn-ghost" onClick={onClose}>取消</button>
              <button className="btn btn-primary-outline" onClick={() => setStep("form")}>
                手动填写 →
              </button>
            </div>
          </div>
        )}

        {/* Step 2: 表单 */}
        {step === "form" && (
          <form className="modal-body" onSubmit={handleSubmit}>
            {/* 基本信息 */}
            <div className="form-section-title">基本信息</div>
            <div className="form-row flex-row-2">
              <div>
                <label className="form-label">配置名称 *</label>
                <input
                  className="form-input"
                  placeholder="例如：DeepSeek 官方"
                  value={form.name}
                  onChange={e => setForm({ ...form, name: e.target.value })}
                />
              </div>
              <div>
                <label className="form-label">平台类型</label>
                <select
                  className="form-input"
                  value={form.platform}
                  onChange={e => setForm({ ...form, platform: e.target.value as Platform })}
                >
                  {(Object.entries(PLATFORM_LABELS) as [Platform, string][]).map(([k, v]) => (
                    <option key={k} value={k}>{v}</option>
                  ))}
                </select>
              </div>
            </div>

            {/* API 配置 */}
            <div className="form-section-title">API 配置</div>
            <div className="form-row">
              <label className="form-label">Base URL（接口地址）</label>
              <input
                className="form-input font-mono"
                placeholder="https://api.example.com/anthropic"
                value={form.base_url}
                onChange={e => setForm({ ...form, base_url: e.target.value })}
              />
              <p className="form-hint">留空使用平台默认地址（Anthropic 官方无需填写）</p>
            </div>
            <div className="form-row flex-row-2">
              <div>
                <label className="form-label">默认模型</label>
                <input
                  className="form-input font-mono"
                  placeholder="claude-opus-4-5"
                  value={form.model_name}
                  onChange={e => setForm({ ...form, model_name: e.target.value })}
                />
              </div>
              {form.api_key_url && (
                <div>
                  <label className="form-label">API Key 申请</label>
                  <a href={form.api_key_url} target="_blank" rel="noreferrer" className="btn btn-ghost btn-sm" style={{ display: "block", textAlign: "center", marginTop: 4 }}>
                    前往申请 ↗
                  </a>
                </div>
              )}
            </div>

            {/* 同步目标 */}
            <div className="form-section-title">同步到哪些工具 *</div>
            <p className="form-hint" style={{ marginTop: -8, marginBottom: 10 }}>
              激活此 Provider 时，将自动写入以下工具的配置文件
            </p>
            <div className="tool-targets-grid">
              {ALL_TOOLS.map(tool => (
                <label key={tool} className={`tool-target-chip ${form.tool_targets.includes(tool) ? "checked" : ""}`}>
                  <input
                    type="checkbox"
                    checked={form.tool_targets.includes(tool)}
                    onChange={() => toggleTool(tool)}
                    style={{ display: "none" }}
                  />
                  <span className="tool-chip-icon">{TOOL_ICONS[tool]}</span>
                  <div>
                    <div className="tool-chip-name">{TOOL_TARGET_LABELS[tool]}</div>
                    <div className="tool-chip-path">{TOOL_TARGET_CONFIG_PATH[tool]}</div>
                  </div>
                  {form.tool_targets.includes(tool) && <span className="tool-chip-check">✓</span>}
                </label>
              ))}
            </div>

            {/* 备注 */}
            <div className="form-row">
              <label className="form-label">备注（可选）</label>
              <textarea
                className="form-input"
                rows={2}
                placeholder="备注信息…"
                value={form.notes}
                onChange={e => setForm({ ...form, notes: e.target.value })}
                style={{ resize: "vertical" }}
              />
            </div>

            {error && <div className="form-error">{error}</div>}

            <div className="modal-footer">
              {!isEditing && (
                <button type="button" className="btn btn-ghost" onClick={() => setStep("preset")}>
                  ← 重选预设
                </button>
              )}
              <button type="button" className="btn btn-ghost" onClick={onClose}>取消</button>
              <button type="submit" className="btn btn-primary" disabled={isSubmitting}>
                {isSubmitting ? "保存中…" : (isEditing ? "保存修改" : "添加 Provider")}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 工具函数：presetId → Platform
// ─────────────────────────────────────────────────────────────────────────────

function presetIdToPlatform(presetId: string): Platform {
  if (presetId.startsWith("anthropic"))  return "anthropic";
  if (presetId.startsWith("openai"))     return "open_ai";
  if (presetId.startsWith("google"))     return "gemini";
  if (presetId.startsWith("deepseek"))   return "deep_seek";
  if (presetId.startsWith("zhipu") || presetId.startsWith("zai")) return "zhipu";
  if (presetId.startsWith("kimi"))       return "moonshot";
  if (presetId.startsWith("minimax"))    return "mini_max";
  if (presetId.startsWith("stepfun"))    return "step_fun";
  if (presetId.startsWith("doubao"))     return "bytedance";
  if (presetId.startsWith("bailian"))    return "aliyun";
  if (presetId.startsWith("aws"))        return "aws_bedrock";
  if (presetId.startsWith("azure"))      return "azure_open_a_i";
  if (presetId.startsWith("nvidia"))     return "nvidia_nim";
  if (presetId.startsWith("openrouter")) return "open_router";
  if (presetId.startsWith("siliconflow"))return "silicon_flow";
  if (presetId.startsWith("github"))     return "copilot";
  return "custom";
}
