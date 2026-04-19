import { useEffect, useState } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig } from "../../types";
import { TOOL_TARGET_LABELS, PLATFORM_LABELS, parseToolTargets } from "../../types";
import { ProviderModal } from "./ProviderModalForm";
import ProviderSnippetModal from "./ProviderSnippetModal";
import { TOOL_ICONS, getProviderPlatformIcon } from "./providerModalShared";
import "./ProviderModal.css";
import "./ProvidersPage.css";

function ProviderDeleteDialog({
  provider,
  onClose,
  onConfirm,
}: {
  provider: ProviderConfig;
  onClose: () => void;
  onConfirm: (id: string) => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>删除 Provider</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <p>确定删除此 Provider 吗？删除后相关工具配置将使用下一个可用的 Provider。</p>
          <div className="modal-footer">
            <button className="btn btn-ghost" onClick={onClose}>取消</button>
            <button className="btn btn-danger" onClick={() => onConfirm(provider.id)}>删除</button>
          </div>
        </div>
      </div>
    </div>
  );
}

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
  onActivate: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className={`provider-list-item ${isSelected ? "selected" : ""}`} onClick={onSelect}>
      <div className="provider-list-item-header">
        <div className="provider-list-item-title">
          <span style={{ fontSize: 16 }}>{getProviderPlatformIcon(provider.platform)}</span>
          <h4>{provider.name}</h4>
          {provider.is_active && (
            <span style={{ color: "var(--color-accent)", fontSize: 12, fontWeight: "bold" }}>
              使用中
            </span>
          )}
        </div>
        <div className="provider-list-item-actions">
          {!provider.is_active && (
            <button
              className="btn btn-primary-outline btn-xs"
              onClick={(event) => {
                event.stopPropagation();
                onActivate();
              }}
              title="使用此配置"
            >
              使用配置
            </button>
          )}
          <button
            className="btn btn-ghost btn-xs"
            onClick={(event) => {
              event.stopPropagation();
              onEdit();
            }}
            title="修改"
          >
            修改
          </button>
          <button
            className="btn btn-danger-ghost btn-xs"
            onClick={(event) => {
              event.stopPropagation();
              onDelete();
            }}
            title="删除"
          >
            删除
          </button>
        </div>
      </div>
      <div className="provider-list-item-desc">
        {PLATFORM_LABELS[provider.platform] ?? provider.platform}
        {provider.model_name ? ` · ${provider.model_name}` : ""}
      </div>
    </div>
  );
}

function ProviderDetailPreview({
  provider,
  onSnippet,
}: {
  provider: ProviderConfig;
  onSnippet: () => void;
}) {
  const targets = parseToolTargets(provider);

  let formattedConfig = "{}";
  try {
    formattedConfig = JSON.stringify(JSON.parse(provider.extra_config || "{}"), null, 2);
  } catch {
    formattedConfig = provider.extra_config || "{}";
  }

  return (
    <div className="animate-fade-in" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div className="preview-header">
        <div
          className="icon-large"
          style={{
            background: provider.icon_color
              ? `${provider.icon_color}22`
              : "var(--color-surface-raised)",
          }}
        >
          {getProviderPlatformIcon(provider.platform)}
        </div>
        <div className="preview-header-content" style={{ flex: 1 }}>
          <h2>{provider.name}</h2>
          <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
            <span className="text-muted" style={{ fontSize: 13 }}>
              {PLATFORM_LABELS[provider.platform] ?? provider.platform}
            </span>
            {provider.is_active && (
              <span
                className="badge"
                style={{
                  background: "var(--gradient-brand)",
                  color: "#fff",
                  padding: "2px 8px",
                  borderRadius: 12,
                  fontSize: 12,
                }}
              >
                当前激活节点
              </span>
            )}
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
              <span className="preview-data-value">
                {provider.api_key_id ? `${provider.api_key_id.substring(0, 8)}...` : "未绑定"}
              </span>
            </div>
          </div>
        </div>

        <div className="preview-section">
          <div className="preview-section-title">通道同步目标</div>
          <div className="targets-chips" style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {targets.length > 0 ? (
              targets.map((target) => (
                <span
                  key={target}
                  className="target-chip active-chip"
                  style={{
                    padding: "6px 12px",
                    background: "var(--color-bg-primary)",
                    border: "1px solid var(--color-border)",
                    borderRadius: 16,
                    fontSize: 13,
                  }}
                >
                  {TOOL_ICONS[target]} {TOOL_TARGET_LABELS[target]}
                </span>
              ))
            ) : (
              <span className="text-muted" style={{ fontSize: 13 }}>无同步目标</span>
            )}
          </div>
        </div>

        <div className="preview-section">
          <div className="preview-section-title">节点底层负载配置快照 (extra_config)</div>
          <pre className="preview-code-block">{formattedConfig}</pre>
        </div>

        {provider.notes && (
          <div className="preview-section">
            <div className="preview-section-title">备注说明</div>
            <p
              style={{
                margin: 0,
                fontSize: 13,
                color: "var(--color-text-secondary)",
                lineHeight: 1.6,
              }}
            >
              {provider.notes}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

export default function ProvidersPage() {
  const { providers, isLoading, error, fetch, switchProvider, deleteProvider } = useProviderStore();
  const [showAdd, setShowAdd] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig | null>(null);
  const [snippetProvider, setSnippetProvider] = useState<ProviderConfig | null>(null);
  const [selectedPreviewId, setSelectedPreviewId] = useState<string | null>(null);
  const [confirmDeleteProvider, setConfirmDeleteProvider] = useState<ProviderConfig | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    void fetch();
  }, [fetch]);

  const handleSwitch = async (id: string) => {
    await switchProvider(id);
  };

  const handleDelete = async (id: string) => {
    await deleteProvider(id);
    if (selectedPreviewId === id) {
      setSelectedPreviewId(null);
    }
    setConfirmDeleteProvider(null);
    setMessage("Provider 已删除");
  };

  const selectedProvider = providers.find((provider) => provider.id === selectedPreviewId) ?? null;

  return (
    <div className="providers-page" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div className="page-header" style={{ marginBottom: 24, flexShrink: 0 }}>
        <div>
          <h1 className="page-title">AI 工具接入层</h1>
          <p className="page-subtitle">
            一条配置，同时同步到多个 AI 编程工具（Claude Code / Codex / Gemini CLI 等）
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => void fetch()} disabled={isLoading}>
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
              {providers.map((provider) => (
                <ProviderListItem
                  key={provider.id}
                  provider={provider}
                  isSelected={selectedPreviewId === provider.id}
                  onSelect={() => setSelectedPreviewId(provider.id)}
                  onActivate={() => void handleSwitch(provider.id)}
                  onEdit={() => setEditingProvider(provider)}
                  onDelete={() => setConfirmDeleteProvider(provider)}
                />
              ))}
            </div>
          )}
        </div>

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
          onSuccess={() => {
            setShowAdd(false);
            void fetch();
          }}
        />
      )}
      {editingProvider && (
        <ProviderModal
          initialProvider={editingProvider}
          onClose={() => setEditingProvider(null)}
          onSuccess={() => {
            setEditingProvider(null);
            void fetch();
          }}
        />
      )}
      {snippetProvider && (
        <ProviderSnippetModal provider={snippetProvider} onClose={() => setSnippetProvider(null)} />
      )}
      {confirmDeleteProvider && (
        <ProviderDeleteDialog
          provider={confirmDeleteProvider}
          onClose={() => setConfirmDeleteProvider(null)}
          onConfirm={(id) => void handleDelete(id)}
        />
      )}
    </div>
  );
}
