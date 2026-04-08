import { useEffect, useState, useMemo } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, ToolTarget } from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH, parseToolTargets, parseToolSpecificConfigs, serializeToolSpecificConfigs } from "../../types";
import ConfigPreview from "./ConfigPreview";
import { ProviderModal } from "./ProviderModal";
import { api } from "../../lib/api";
import { CheckCircle2, Circle } from "lucide-react";
import "./ToolSyncPage.css";
import "./ProviderModal.css";

const ALL_TOOLS: ToolTarget[] = [
  "claude_code",
  "codex",
  "gemini_cli",
  "open_code",
  "open_claw",
  "aider"
];

const TOOL_ICONS: Record<ToolTarget, string> = {
  claude_code: "🤖",
  codex:       "🧠",
  gemini_cli:  "✨",
  open_code:   "🌐",
  open_claw:   "🦞",
  aider:       "💻",
};

function ProviderRow({ 
  provider, 
  isActive, 
  localConfig,
  onChangeModel,
  onEdit,
  onDelete,
  onActivate
}: { 
  provider: ProviderConfig; 
  isActive: boolean; 
  localConfig: any;
  onChangeModel: (model: string) => void;
  onSaveModel: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onActivate: () => void;
}) {
  return (
    <div 
      className={`provider-card ${isActive ? "active" : ""}`}
      onClick={!isActive ? onActivate : undefined}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <div className="provider-check">
          {isActive ? <CheckCircle2 size={22} fill="var(--color-primary)" color="var(--color-surface, #fff)" /> : <Circle size={22} color="var(--color-text-tertiary)" />}
        </div>
        
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
          <div style={{ fontWeight: 600, fontSize: 15, color: isActive ? "var(--color-primary)" : "var(--color-text)" }}>
            {provider.name}
          </div>
          <div style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
            默认模型: <span style={{ fontFamily: "monospace", display: "inline-block", padding: "2px 6px", background: "var(--color-bg-primary)", borderRadius: 4 }}>{provider.model_name || "无"}</span>
          </div>
        </div>
        
        {isActive && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6, width: 220, marginRight: 16 }} onClick={e => e.stopPropagation()}>
            <label style={{ fontSize: 11, color: "var(--color-text-secondary)", fontWeight: 500 }}>
              独立模型劫持定义 (仅作用于当前终端)
            </label>
            <input 
              className="premium-input form-input"
              style={{ padding: "6px 12px", fontSize: 13, borderRadius: 8 }}
              placeholder={provider.model_name || "自定义模型..."}
              value={localConfig?.model_name || ""}
              onChange={(e) => onChangeModel(e.target.value)}
              onBlur={() => onSaveModel()}
            />
          </div>
        )}
        
        {/* 操作区 */}
        <div style={{ display: "flex", gap: "8px" }} className="row-actions" onClick={e => e.stopPropagation()}>
          <button className="btn btn-ghost btn-sm" onClick={onEdit}>修改</button>
          <button className="btn btn-danger-ghost btn-sm" onClick={onDelete}>删除</button>
        </div>
      </div>
    </div>
  );
}

export default function ToolSyncPage() {
  const { providers, isLoading, fetch, deleteProvider } = useProviderStore();
  const [activeTab, setActiveTab] = useState<ToolTarget>("claude_code");
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig | undefined>(undefined);
  
  const [localProviders, setLocalProviders] = useState<ProviderConfig[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetch();
  }, [fetch]);

  useEffect(() => {
    setLocalProviders([...providers].sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0)));
  }, [providers]);

  // 获取当前标签页专属的 providers（避免混合显示）
  const tabProviders = useMemo(() => {
    return localProviders.filter(p => parseToolTargets(p).includes(activeTab));
  }, [localProviders, activeTab]);

  // 当前标签页激活的 provider
  const activeProvider = useMemo(() => {
    return tabProviders.find(p => p.is_active);
  }, [tabProviders]);

  const handleActivate = async (selectedProviderId: string) => {
    // 乐观 UI 更新
    setLocalProviders(prev => prev.map(p => {
      if (parseToolTargets(p).includes(activeTab)) {
        return { ...p, is_active: p.id === selectedProviderId };
      }
      return p;
    }));

    try {
      // 触发后端：同目标隔离式单选开关
      await api.providers.switch(selectedProviderId);
      // fetch(); // (静默，由外部处理或依赖 websocket 更新)
    } catch (e) {
      console.error(e);
      await fetch(); // 如果失败，回滚
    }
  };

  const handleDelete = async (providerId: string) => {
    if (!confirm("确定要删除此配置源吗？")) return;
    await deleteProvider(providerId);
    await fetch();
  };

  const handleChangeOverrideModel = (providerId: string, overrideModel: string) => {
    setLocalProviders(prev => prev.map(p => {
      if (p.id === providerId) {
        const configs = parseToolSpecificConfigs(p);
        if (!configs[activeTab]) configs[activeTab] = {} as any;
        (configs[activeTab] as any).model_name = overrideModel;
        return { ...p, extra_config: serializeToolSpecificConfigs(p.extra_config, configs) };
      }
      return p;
    }));
  };

  const handleSaveModel = async (providerId: string) => {
    const p = localProviders.find(p => p.id === providerId);
    if (!p) return;
    try {
      await api.providers.update(p);
      await fetch();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="tool-sync-page animate-fade-in" style={{ padding: 32, height: "100%", display: "flex", flexDirection: "column", overflowY: "auto", background: "var(--color-bg-primary)" }}>
      <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
        
        {/* 精美的顶部 Tabs 区域 */}
        <div style={{ marginBottom: 32, flexShrink: 0 }}>
          <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text-secondary)", marginBottom: 12, textTransform: "uppercase", letterSpacing: "0.5px" }}>
            选择需要注入配置的终端
          </div>
          <div className="premium-tabs-container">
            {ALL_TOOLS.map(tool => {
              const isActive = activeTab === tool;
              return (
                <button
                  key={tool}
                  className={`premium-tab ${isActive ? "active" : ""}`}
                  onClick={() => setActiveTab(tool)}
                >
                  <span className="tab-icon">{TOOL_ICONS[tool]}</span>
                  <span className="tab-label">{TOOL_TARGET_LABELS[tool]}</span>
                </button>
              )
            })}
          </div>
        </div>

        {/* 下方 面板内容 */}
        <div style={{ flex: 1, paddingRight: 8, overflowY: "auto" }}>
          
          {/* ActionBar */}
          <div style={{ display: "flex", justifyItems: "center", justifyContent: "space-between", alignItems: "center", marginBottom: 20 }}>
            <div>
              <h2 style={{ margin: "0 0 6px 0", fontSize: 18, fontWeight: 600, display: "flex", alignItems: "center", gap: 12 }}>
                当前通道：{TOOL_TARGET_LABELS[activeTab]}
              </h2>
              <div style={{ fontSize: 13, color: "var(--color-text-tertiary)", fontFamily: "monospace", display: "flex", alignItems: "center", gap: 8 }}>
                注入锚点:
                <span style={{ background: "var(--color-surface)", padding: "2px 8px", borderRadius: 6, border: "1px solid var(--color-border)", color: "var(--color-text-secondary)" }}>
                  {TOOL_TARGET_CONFIG_PATH[activeTab]}
                </span>
              </div>
            </div>
            <div style={{ display: "flex", gap: "12px", height: "40px" }}>
              <button className="btn btn-primary premium-btn" onClick={() => setShowProviderModal(true)}>
                ＋ 新增配置源
              </button>
            </div>
          </div>

          <div style={{ display: "flex", gap: "32px", alignItems: "flex-start", marginTop: 12 }}>
            
            {/* 左侧：全新配置源单选列表 */}
            <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 12 }}>
              {tabProviders.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-icon">🔌</div>
                  <h3>配置列表为空</h3>
                  <p>您需要为该终端添加一个配置源，例如官方直连或您的聚合网关。</p>
                  <button className="btn btn-primary" onClick={() => setShowProviderModal(true)}>
                    + 新增配置源
                  </button>
                </div>
              ) : (
                tabProviders.map((p) => {
                  const isActive = p.is_active;
                  const extra = parseToolSpecificConfigs(p)[activeTab];
                  return (
                    <ProviderRow
                      key={p.id}
                      provider={p}
                      isActive={isActive}
                      localConfig={extra}
                      onChangeModel={(val) => handleChangeOverrideModel(p.id, val)}
                      onSaveModel={() => handleSaveModel(p.id)}
                      onEdit={() => { setEditingProvider(p); setShowProviderModal(true); }}
                      onDelete={() => handleDelete(p.id)}
                      onActivate={() => handleActivate(p.id)}
                    />
                  );
                })
              )}
            </div>

            {/* 右侧：配置文件预览 */}
            {activeProvider && (
              <div style={{ width: "450px", flexShrink: 0, position: "sticky", top: 0 }}>
                <div style={{ 
                  background: "var(--color-surface)", 
                  borderRadius: 16, 
                  border: "1px solid var(--color-border)", 
                  overflow: "hidden",
                  boxShadow: "0 12px 32px rgba(0,0,0,0.06)"
                }}>
                  <div style={{ padding: "16px 20px", borderBottom: "1px solid var(--color-border)", background: "var(--color-bg-primary)" }}>
                    <div style={{ fontSize: 13, fontWeight: 700, color: "var(--color-text-secondary)", letterSpacing: "0.5px" }}>
                      实际注入配置文件预览
                    </div>
                  </div>
                  <ConfigPreview
                    baseUrl={activeProvider.base_url || "https://api.anthropic.com"}
                    apiKey={activeProvider.api_key_id ? "sk-ais-*********" : ""}
                    toolTargets={[activeTab]}
                    toolConfigs={{
                      [activeTab]: parseToolSpecificConfigs(activeProvider)[activeTab]
                    }}
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
      
      {showProviderModal && (
        <ProviderModal
          fixedTool={activeTab}
          initialProvider={editingProvider}
          onClose={() => { setShowProviderModal(false); setEditingProvider(undefined); }}
          onSuccess={() => {
            setShowProviderModal(false);
            setEditingProvider(undefined);
            fetch();
          }}
        />
      )}
    </div>
  );
}
