import React, { useEffect, useState, useMemo } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, ToolTarget, Platform } from "../../types";
import {
  TOOL_TARGET_LABELS,
  PLATFORM_LABELS,
  parseToolTargets,
} from "../../types";
import {
  filterPresetsByTool,
  groupPresetsByToolAndCategory,
  CATEGORY_LABELS,
  type ProviderPreset,
  type ProviderCategory,
} from "../../data/providerPresets";
import { api } from "../../lib/api";
import { ProviderAdvancedConfig, type ProviderExtraConfig } from "./ProviderAdvancedConfig";
import { JsonConfigEditor } from "./JsonConfigEditor";
import ProviderSnippetModal from "./ProviderSnippetModal";
import { AnthropicFormFields } from "./forms/AnthropicFormFields";
import { OpenAIFormFields } from "./forms/OpenAIFormFields";
import { GeminiFormFields } from "./forms/GeminiFormFields";
import "./ProviderModal.css";

const ALL_TOOLS: ToolTarget[] = ["claude_code", "codex", "gemini_cli", "open_code", "open_claw", "aider"];

// ─────────────────────────────────────────────────────────────────────────────
// Provider Modal（新增 + 编辑）
// ─────────────────────────────────────────────────────────────────────────────

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
  const [snippetProvider, setSnippetProvider] = useState<ProviderConfig | null>(null);
  const [selectedPreviewId, setSelectedPreviewId] = useState<string | null>(null);
  const [confirmDeleteProvider, setConfirmDeleteProvider] = useState<ProviderConfig | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => { fetch(); }, [fetch]);

  const handleSwitch = async (id: string) => {
    await switchProvider(id);
  };

  const handleDelete = async (id: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    await deleteProvider(id);
    if (selectedPreviewId === id) setSelectedPreviewId(null);
    setConfirmDeleteProvider(null);
    setMessage("Provider 已删除");
  };

  const selectedProvider = providers.find(p => p.id === selectedPreviewId);

  return (
    <div className="providers-page" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div className="page-header" style={{ marginBottom: 24, flexShrink: 0 }}>
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
      {message && <div className="alert alert-info" style={{ marginBottom: 16 }}>{message}</div>}

      <div className="providers-split-layout">
        {/* 左侧列表 */}
        <div className="providers-list-panel">
          {isLoading && providers.length === 0 ? (
            <div className="empty-state" style={{ height: "100%" }}>
              <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
              <span>加载中...</span>
            </div>
          ) : providers.length === 0 ? (
            <div className="empty-state" style={{ height: "100%", padding: 24 }}>
              <div className="empty-state-icon">⚡</div>
              <h3 style={{ color: "var(--color-text-secondary)" }}>暂无 Provider</h3>
              <p style={{ fontSize: 13, marginBottom: 16 }}>点击右上角快速接入</p>
            </div>
          ) : (
            <div className="list-group">
              {providers.map((p) => (
                <ProviderListItem
                  key={p.id}
                  provider={p}
                  isSelected={selectedPreviewId === p.id}
                  onSelect={() => setSelectedPreviewId(p.id)}
                  onActivate={(e) => { e.stopPropagation(); handleSwitch(p.id); }}
                  onEdit={(e) => { e.stopPropagation(); setEditingProvider(p); }}
                  onDelete={(e) => { e.stopPropagation(); setConfirmDeleteProvider(p); }}
                />
              ))}
            </div>
          )}
        </div>

        {/* 右侧详情 */}
        <div className="providers-preview-panel">
          {selectedProvider ? (
            <ProviderDetailPreview 
              provider={selectedProvider} 
              onSnippet={() => setSnippetProvider(selectedProvider)}
            />
          ) : (
            <div className="preview-empty-state">
              <div className="preview-empty-state-icon">💡</div>
              <h3>未选择任何配置</h3>
              <p>请在左侧点击节点以查看预览详情</p>
            </div>
          )}
        </div>
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
      {snippetProvider && (
        <ProviderSnippetModal
          provider={snippetProvider}
          onClose={() => setSnippetProvider(null)}
        />
      )}
      {confirmDeleteProvider && (
        <div className="modal-overlay" onClick={() => setConfirmDeleteProvider(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>删除 Provider</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDeleteProvider(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>确定删除此 Provider 吗？删除后相关工具配置将使用下一个可用的 Provider。</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmDeleteProvider(null)}>取消</button>
                <button className="btn btn-danger" onClick={() => handleDelete(confirmDeleteProvider.id)}>删除</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Card
// ─────────────────────────────────────────────────────────────────────────────

function ProviderListItem({
  provider,
  isSelected,
  onSelect,
  onActivate,
  onEdit,
  onDelete,
}: {
  provider: ProviderConfig;
  isSelected: boolean;
  onSelect: () => void;
  onActivate: (e: React.MouseEvent) => void;
  onEdit: (e: React.MouseEvent) => void;
  onDelete: (e: React.MouseEvent) => void;
}) {
  return (
    <div className={`provider-list-item ${isSelected ? "selected" : ""}`} onClick={onSelect}>
      <div className="provider-list-item-header">
        <div className="provider-list-item-title">
          <span style={{ fontSize: 16 }}>
            {provider.platform === "anthropic" ? "🟠" :
             provider.platform === "open_ai"   ? "🟢" :
             provider.platform === "gemini"     ? "🔵" :
             provider.platform === "deep_seek"  ? "🔷" :
             provider.platform === "open_router"? "🟣" : "⚙️"}
          </span>
          <h4>{provider.name}</h4>
          {provider.is_active && (
            <span style={{ color: "var(--color-accent)", fontSize: 12, fontWeight: "bold" }}>使用中</span>
          )}
        </div>
        <div className="provider-list-item-actions">
          {!provider.is_active && (
            <button className="btn btn-primary-outline btn-xs" onClick={onActivate} title="使用此配置">使用配置</button>
          )}
          <button className="btn btn-ghost btn-xs" onClick={onEdit} title="修改">修改</button>
          <button className="btn btn-danger-ghost btn-xs" onClick={onDelete} title="删除">删除</button>
        </div>
      </div>
      <div className="provider-list-item-desc">
        {PLATFORM_LABELS[provider.platform] ?? provider.platform} 
        {provider.model_name ? ` · ${provider.model_name}` : ""}
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Detail Preview
// ─────────────────────────────────────────────────────────────────────────────

function ProviderDetailPreview({ provider, onSnippet }: { provider: ProviderConfig; onSnippet: () => void; }) {
  const targets = parseToolTargets(provider);

  let formattedConfig = "{}";
  try {
    formattedConfig = JSON.stringify(JSON.parse(provider.extra_config || "{}"), null, 2);
  } catch(e) {
    formattedConfig = provider.extra_config || "{}";
  }

  return (
    <div className="animate-fade-in" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div className="preview-header">
        <div className="icon-large" style={{ background: provider.icon_color ? `${provider.icon_color}22` : "var(--color-surface-raised)" }}>
          {provider.platform === "anthropic" ? "🟠" :
           provider.platform === "open_ai"   ? "🟢" :
           provider.platform === "gemini"     ? "🔵" :
           provider.platform === "deep_seek"  ? "🔷" :
           provider.platform === "open_router"? "🟣" : "⚙️"}
        </div>
        <div className="preview-header-content" style={{ flex: 1 }}>
          <h2>{provider.name}</h2>
          <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
            <span className="text-muted" style={{ fontSize: 13 }}>
              {PLATFORM_LABELS[provider.platform] ?? provider.platform}
            </span>
            {provider.is_active && <span className="badge" style={{ background: "var(--gradient-brand)", color: "#fff", padding: "2px 8px", borderRadius: 12, fontSize: 12 }}>当前激活节点</span>}
          </div>
        </div>
        <div>
          <button className="btn btn-ghost" onClick={onSnippet}>( &lt;/&gt; ) 获取代码</button>
        </div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", paddingRight: 8 }}>
        <div className="preview-section">
          <div className="preview-section-title">核心属性</div>
          <div className="preview-data-grid">
            <div className="preview-data-item">
              <span className="preview-data-label">默认模型</span>
              <span className="preview-data-value">{provider.model_name || "未指定"}</span>
            </div>
            <div className="preview-data-item" style={{ gridColumn: "span 2" }}>
              <span className="preview-data-label">接口地址 (Base URL)</span>
              <span className="preview-data-value" style={{ fontFamily: "var(--font-mono)" }}>
                {provider.base_url || "使用平台默认地址"}
              </span>
            </div>
            <div className="preview-data-item">
              <span className="preview-data-label">凭证 ID</span>
              <span className="preview-data-value">{provider.api_key_id ? provider.api_key_id.substring(0,8)+"..." : "未绑定"}</span>
            </div>
          </div>
        </div>

        <div className="preview-section">
          <div className="preview-section-title">通道同步目标</div>
          <div className="targets-chips" style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {targets.length > 0 ? targets.map(t => (
              <span key={t} className="target-chip active-chip" style={{ padding: "6px 12px", background: "var(--color-bg-primary)", border: "1px solid var(--color-border)", borderRadius: 16, fontSize: 13 }}>
                {TOOL_ICONS[t]} {TOOL_TARGET_LABELS[t]}
              </span>
            )) : <span className="text-muted" style={{ fontSize: 13 }}>无同步目标</span>}
          </div>
        </div>

        <div className="preview-section">
          <div className="preview-section-title">节点底层负载配置快照 (extra_config)</div>
          <pre className="preview-code-block">{formattedConfig}</pre>
        </div>
        
        {provider.notes && (
          <div className="preview-section">
             <div className="preview-section-title">备注说明</div>
             <p style={{ margin: 0, fontSize: 13, color: "var(--color-text-secondary)", lineHeight: 1.6 }}>{provider.notes}</p>
          </div>
        )}
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
  extra_config: string; // JSON string payload for Advanced settings
}

export function ProviderModal({
  initialProvider,
  onClose,
  onSuccess,
  fixedTool,
}: {
  initialProvider?: ProviderConfig;
  onClose: () => void;
  onSuccess: () => void;
  fixedTool?: ToolTarget;
}) {
  const { add, update } = useProviderStore();

  const [step, setStep] = useState<"preset" | "form">(initialProvider ? "form" : "preset");
  const [presetSearch, setPresetSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<ProviderCategory | "all">("all");
  const [activeTab, setActiveTab] = useState<'basic' | 'advanced'>('basic');

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
        extra_config:  initialProvider.extra_config ?? "{}",
      };
    }
    return {
      name: "", platform: "custom", base_url: "", model_name: "",
      api_key_value: "", tool_targets: fixedTool ? [fixedTool] : ["claude_code"],
      website_url: "", api_key_url: "", notes: "", extra_config: "{}"
    };
  });

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [modelFetchError, setModelFetchError] = useState("");

  // ── 预设过滤 ──────────────────────────────────────────────────────────────

  const groupedPresets = useMemo(() => groupPresetsByToolAndCategory(fixedTool), [fixedTool]);
  const filteredPresets = useMemo(() => {
    let base = selectedCategory === "all"
      ? filterPresetsByTool(fixedTool)
      : groupedPresets[selectedCategory] ?? [];

    if (!presetSearch.trim()) return base;
    const q = presetSearch.toLowerCase();
    return base.filter(p =>
      p.name.toLowerCase().includes(q) ||
      (p.defaultBaseUrl?.toLowerCase().includes(q) ?? false),
    );
  }, [selectedCategory, presetSearch, groupedPresets, fixedTool]);

  const applyPreset = (preset: ProviderPreset) => {
    setForm(f => ({
      ...f,
      name:       preset.name,
      platform:   preset.platform as Platform,
      base_url:   preset.defaultBaseUrl ?? "",
      model_name: preset.defaultModel ?? "",
      website_url: preset.websiteUrl ?? "",
      api_key_url: preset.apiKeyUrl ?? "",
      notes:      preset.notes ?? "",
      extra_config: preset.settingsConfig ? JSON.stringify(preset.settingsConfig, null, 2) : "{}",
    }));
    setStep("form");
  };

  // 临时解析额外的 advanced 面板状态
  let advancedCfg: ProviderExtraConfig = {};
  try {
    advancedCfg = JSON.parse(form.extra_config || "{}");
  } catch(e) {}

  // ── 表单提交 ──────────────────────────────────────────────────────────────

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) { setError("名称不能为空"); return; }
    if (form.tool_targets.length === 0) { setError("请至少选择一个同步目标"); return; }

    setIsSubmitting(true);
    setError("");
    try {
      let finalApiKeyId = initialProvider?.api_key_id ?? undefined;
      
      // 如果用户在表单上输入了 API Key，静默托管入库中心并在 Provider 关联指针。
      if (form.api_key_value.trim()) {
        const newKey = await api.keys.add({
          name: form.name.trim() + " (Auto Key)",
          platform: form.platform,
          secret: form.api_key_value.trim(),
          base_url: form.base_url.trim() || undefined,
        });
        finalApiKeyId = newKey.id;
      }

      const payload: any = {
        id:           initialProvider?.id || crypto.randomUUID(),
        name:         form.name.trim(),
        platform:     form.platform,
        api_key_id:   finalApiKeyId,
        base_url:     form.base_url.trim() || null,
        model_name:   form.model_name.trim(),
        is_active:    initialProvider?.is_active ?? false,
        tool_targets: JSON.stringify(form.tool_targets),
        website_url:  form.website_url.trim() || null,
        api_key_url:  form.api_key_url.trim() || null,
        notes:        form.notes.trim() || null,
        extra_config: form.extra_config.trim() || "{}",
        created_at:   initialProvider?.created_at || new Date().toISOString(),
        updated_at:   new Date().toISOString(),
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

  const handleFetchModels = async () => {
    setIsFetchingModels(true);
    setModelFetchError("");
    try {
      const models = await api.providers.fetchModels({
        platform: form.platform,
        base_url: form.base_url.trim() || undefined,
        api_key_value: form.api_key_value.trim() || undefined,
        api_key_id: initialProvider?.api_key_id,
      });
      setModelOptions(models);
      if (!form.model_name.trim() && models.length > 0) {
        setForm((prev) => ({ ...prev, model_name: models[0] }));
      }
    } catch (err) {
      setModelFetchError(String(err));
    } finally {
      setIsFetchingModels(false);
    }
  };

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{isEditing ? `编辑 ${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : ""}节点配置` : (step === "preset" ? `${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : "全局"}通道快速模板` : `新增 ${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : ""}节点`)}</h2>
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
              {filteredPresets.map(preset => {
                const catColors = { relay: '#8B5CF6', official: '#3B82F6', custom: '#6B7280' };
                const ac = catColors[preset.category] || '#6B7280';
                return (
                  <button
                    key={preset.presetId}
                    className={`preset-card ${preset.category === 'relay' ? 'preset-card-premium' : ''}`}
                    style={preset.category !== 'relay' ? { borderLeftColor: ac, borderLeftWidth: 3 } : {}}
                    onClick={() => applyPreset(preset)}
                  >
                    <div className="preset-name">
                      {preset.category === 'relay' && <span style={{ marginRight: 6 }}>👑</span>}
                      {preset.name}
                    </div>
                    <div className="preset-meta" style={{ color: ac, fontWeight: 600, fontSize: 11 }}>
                      {CATEGORY_LABELS[preset.category]}
                    </div>
                    {preset.defaultBaseUrl && (
                      <div className="preset-url font-mono">{preset.defaultBaseUrl}</div>
                    )}
                  </button>
                );
              })}
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
            <div className="form-tabs">
              <div className={`form-tab ${activeTab === 'basic' ? 'active' : ''}`} onClick={() => setActiveTab('basic')}>基础设定</div>
              <div className={`form-tab ${activeTab === 'advanced' ? 'active' : ''}`} onClick={() => setActiveTab('advanced')}>调优与进阶特性</div>
            </div>

            {activeTab === 'basic' && (
              <>
                {/* 1. 基本信息 */}
                <div className="form-grid-2" style={{ marginBottom: 16 }}>
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

                {/* 2. API 配置 */}
                <div className="form-row">
                  <label className="form-label">Base URL（接口地址）</label>
                  <input
                    className="form-input font-mono"
                    placeholder="https://api.example.com/anthropic"
                    value={form.base_url}
                    onChange={e => setForm({ ...form, base_url: e.target.value })}
                  />
                  <p className="form-hint" style={{ marginTop: 4 }}>留空使用平台默认地址（Anthropic 官方无需填写）</p>
                </div>
                
                <div className="form-row">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
                    <label className="form-label" style={{ marginBottom: 0 }}>API Key 凭证</label>
                    {form.api_key_url && (
                      <a href={form.api_key_url} target="_blank" rel="noreferrer" className="text-accent" style={{ fontSize: 12, textDecoration: 'none', fontWeight: 500 }}>
                        获取专属 Key ↗
                      </a>
                    )}
                  </div>
                  <input
                    type="password"
                    className="form-input font-mono"
                    placeholder={isEditing ? "(留空保持原凭证不变)" : "sk-..."}
                    value={form.api_key_value}
                    onChange={e => setForm({ ...form, api_key_value: e.target.value })}
                  />
                  <p className="form-hint" style={{ marginTop: 4 }}>提交后将被加密保存至凭证中心</p>
                </div>

                {/* 3. 默认模型 */}
                <div className="form-row">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                    <label className="form-label" style={{ marginBottom: 0 }}>默认模型</label>
                    <button
                      type="button"
                      className="btn btn-ghost btn-xs"
                      onClick={handleFetchModels}
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
                    onChange={e => setForm({ ...form, model_name: e.target.value })}
                  />
                  {modelOptions.length > 0 && (
                    <select
                      className="form-input"
                      value={form.model_name}
                      onChange={e => setForm({ ...form, model_name: e.target.value })}
                      style={{ marginTop: 8 }}
                    >
                      {modelOptions.map((model) => (
                        <option key={model} value={model}>{model}</option>
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
                    onChange={e => setForm({ ...form, notes: e.target.value })}
                    style={{ resize: "vertical" }}
                  />
                </div>

                {/* 同步目标 */}
                {!fixedTool && (
                  <div style={{ marginTop: 24, marginBottom: 16 }}>
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
                          {TOOL_TARGET_LABELS[tool]}
                        </label>
                      ))}
                    </div>
                  </div>
                )}
              </>
            )}

            {activeTab === 'advanced' && (
              <>
                {/* 1. 调参设定 */}
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
                      onChange={e => {
                        const extra = { ...advancedCfg };
                        if (e.target.value === "") delete extra.temperature;
                        else extra.temperature = parseFloat(e.target.value);
                        setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                      }}
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
                      onChange={e => {
                        const extra = { ...advancedCfg };
                        if (e.target.value === "") delete extra.maxTokens;
                        else extra.maxTokens = parseInt(e.target.value);
                        setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                      }}
                    />
                  </div>
                </div>

                {/* 当该提供商是为 Claude Code 服务时，显示大小杯模型覆盖 */}
                {(fixedTool === "claude_code" || (form.tool_targets && form.tool_targets.includes("claude_code"))) && (
                  <div style={{ marginBottom: 16, padding: '16px', background: 'var(--color-surface-raised)', borderRadius: 12, border: '1px solid var(--color-border)' }}>
                    <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>Claude 大小杯模型映射 (Overrides)</div>
                    
                    <div className="form-grid-2">
                      <div className="form-row">
                        <label className="form-label">Haiku 小模型</label>
                        <input 
                          className="form-input font-mono"
                          placeholder="claude-3-5-haiku-20241022"
                          value={advancedCfg.tool_configs?.claude_code?.haikuModel || ''}
                          onChange={e => {
                            const tc = advancedCfg.tool_configs || {};
                            const cc = tc.claude_code || {};
                            const extra = { ...advancedCfg, tool_configs: { ...tc, claude_code: { ...cc, haikuModel: e.target.value || undefined } } };
                            setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                          }}
                        />
                      </div>
                      <div className="form-row">
                        <label className="form-label">Sonnet 中模型</label>
                        <input 
                          className="form-input font-mono"
                          placeholder="claude-3-5-sonnet-20241022"
                          value={advancedCfg.tool_configs?.claude_code?.sonnetModel || ''}
                          onChange={e => {
                            const tc = advancedCfg.tool_configs || {};
                            const cc = tc.claude_code || {};
                            const extra = { ...advancedCfg, tool_configs: { ...tc, claude_code: { ...cc, sonnetModel: e.target.value || undefined } } };
                            setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                          }}
                        />
                      </div>
                    </div>
                    
                    <div className="form-grid-2">
                      <div className="form-row">
                        <label className="form-label">Opus 大模型</label>
                        <input 
                          className="form-input font-mono"
                          placeholder="claude-3-opus-20240229"
                          value={advancedCfg.tool_configs?.claude_code?.opusModel || ''}
                          onChange={e => {
                            const tc = advancedCfg.tool_configs || {};
                            const cc = tc.claude_code || {};
                            const extra = { ...advancedCfg, tool_configs: { ...tc, claude_code: { ...cc, opusModel: e.target.value || undefined } } };
                            setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                          }}
                        />
                      </div>
                      <div className="form-row">
                        <label className="form-label">Reasoning 推理模型</label>
                        <input 
                          className="form-input font-mono"
                          placeholder="claude-3-7-sonnet-20250219"
                          value={advancedCfg.tool_configs?.claude_code?.reasoningModel || ''}
                          onChange={e => {
                            const tc = advancedCfg.tool_configs || {};
                            const cc = tc.claude_code || {};
                            const extra = { ...advancedCfg, tool_configs: { ...tc, claude_code: { ...cc, reasoningModel: e.target.value || undefined } } };
                            setForm({ ...form, extra_config: JSON.stringify(extra, null, 2) });
                          }}
                        />
                      </div>
                    </div>
                    <div className="form-hint" style={{ marginTop: 2 }}>当对应场景触发时，终端将使用以上设定替代官方硬编码来向本节点发送请求。</div>
                  </div>
                )}



                {/* 高阶网络与平台专属特性 (Advanced & Quirks) */}
                <div style={{ marginTop: 8, marginBottom: 16 }}>
                  {/* 网络与基础高级配置（代理、测速等通用） */}
                  <ProviderAdvancedConfig 
                    value={advancedCfg}
                    onChange={cfg => setForm({ ...form, extra_config: JSON.stringify(cfg, null, 2) })}
                  />

                  {/* 平台专属 Quirks 卡片加载 */}
                  {form.platform === 'anthropic' && (
                    <div style={{ padding: '0 16px', background: 'var(--color-surface)', border: '1px solid var(--color-border)', borderRadius: 12, marginTop: 12, paddingBottom: 16 }}>
                      <AnthropicFormFields 
                        value={advancedCfg} 
                        onChange={cfg => setForm({ ...form, extra_config: JSON.stringify(cfg, null, 2) })} 
                      />
                    </div>
                  )}
                  {form.platform === 'open_ai' && (
                    <div style={{ padding: '0 16px', background: 'var(--color-surface)', border: '1px solid var(--color-border)', borderRadius: 12, marginTop: 12, paddingBottom: 16 }}>
                       <OpenAIFormFields 
                        value={advancedCfg} 
                        onChange={cfg => setForm({ ...form, extra_config: JSON.stringify(cfg, null, 2) })} 
                      />
                    </div>
                  )}
                  {form.platform === 'gemini' && (
                    <div style={{ padding: '0 16px', background: 'var(--color-surface)', border: '1px solid var(--color-border)', borderRadius: 12, marginTop: 12, paddingBottom: 16 }}>
                       <GeminiFormFields 
                        value={advancedCfg} 
                        onChange={cfg => setForm({ ...form, extra_config: JSON.stringify(cfg, null, 2) })} 
                      />
                    </div>
                  )}

                   {/* 保底的 JSON 编辑器 - 仅开发模式或自定义平台 */}
                  {(form.platform === 'custom') && (
                    <div style={{ padding: '0 16px', background: 'var(--color-surface)', border: '1px solid var(--color-border)', borderRadius: 12, marginTop: 12, paddingBottom: 16 }}>
                       <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>底层配置重写 (Advanced)</div>
                       <JsonConfigEditor 
                          value={form.extra_config} 
                          onChange={val => setForm({ ...form, extra_config: val })}
                       />
                    </div>
                  )}
                </div>
              </>
            )}

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

