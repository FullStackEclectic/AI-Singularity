import { useEffect, useState, useMemo } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, ToolTarget } from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH, parseToolTargets, parseToolSpecificConfigs, serializeToolSpecificConfigs } from "../../types";
import ConfigPreview from "./ConfigPreview";
import { ProviderModal } from "./ProvidersPage";
import { api } from "../../lib/api";
import "./ToolSyncPage.css";
import "./ProvidersPage.css";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import "./ToolSyncPage.css";

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

// --- 可拖拽的备选源条目 (复用) ---
function SortableProviderRow({ 
  provider, 
  index, 
  isActive, 
  onToggle, 
  localConfig,
  onChangeModel
}: { 
  provider: ProviderConfig; 
  index: number; 
  isActive: boolean; 
  onToggle: (v: boolean) => void;
  localConfig: any;
  onChangeModel: (model: string) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: provider.id,
  });
  
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 1 : 0,
    backgroundColor: isActive ? "var(--color-primary-alpha)" : "var(--color-bg-primary)",
  };

  return (
    <div ref={setNodeRef} style={style} className={`provider-row ${isDragging ? "dragging" : ""}`} {...attributes} {...listeners}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "12px", borderBottom: "1px solid var(--color-border)" }}>
        <input 
          type="checkbox" 
          checked={isActive} 
          onChange={(e) => onToggle(e.target.checked)} 
          style={{ cursor: "pointer", accentColor: "var(--color-primary)" }}
        />
        {isActive && (
          <span style={{ 
            background: index === 0 ? "goldenrod" : index === 1 ? "silver" : "#cd7f32", 
            color: "#fff", padding: "2px 6px", borderRadius: 4, fontWeight: "bold", fontSize: 10 
          }}>
            P{index + 1}
          </span>
        )}
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600 }}>{provider.name}</div>
          <div style={{ fontSize: 11, color: "var(--color-text-tertiary)" }}>
            默认模型: {provider.model_name || "无"}
          </div>
        </div>
        
        {isActive && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4, width: 180 }}>
            <label style={{ fontSize: 10, color: "var(--color-text-tertiary)" }}>独立劫持模型定义</label>
            <input 
              className="form-input"
              style={{ padding: "4px 8px", fontSize: 12 }}
              placeholder={provider.model_name || "自定义模型..."}
              value={localConfig?.model_name || ""}
              onChange={(e) => onChangeModel(e.target.value)}
              onClick={e => e.stopPropagation()}
            />
          </div>
        )}
      </div>
    </div>
  );
}

export default function ToolSyncPage() {
  const { providers, isLoading, fetch } = useProviderStore();
  const [activeTab, setActiveTab] = useState<ToolTarget>("claude_code");
  const [proxyTakeover, setProxyTakeover] = useState(false);
  const [showProviderModal, setShowProviderModal] = useState(false);
  
  // 用于拖拽的本地状
  const [localProviders, setLocalProviders] = useState<ProviderConfig[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetch();
  }, [fetch]);

  useEffect(() => {
    setLocalProviders([...providers].sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0)));
  }, [providers]);

  // 取当前 Tab 下绑定的 Providers
  const activeProviders = useMemo(() => {
    return localProviders.filter(p => parseToolTargets(p).includes(activeTab));
  }, [localProviders, activeTab]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      const oldIndex = localProviders.findIndex((p) => p.id === active.id);
      const newIndex = localProviders.findIndex((p) => p.id === over.id);
      setLocalProviders(arrayMove(localProviders, oldIndex, newIndex));
    }
  };

  const handleToggle = (providerId: string, checked: boolean) => {
    setLocalProviders(prev => prev.map(p => {
      if (p.id === providerId) {
        let targets = parseToolTargets(p);
        if (checked && !targets.includes(activeTab)) targets.push(activeTab);
        if (!checked) targets = targets.filter(t => t !== activeTab);
        p.tool_targets = JSON.stringify(targets);
      }
      return p;
    }));
  };

  const handleChangeOverrideModel = (providerId: string, overrideModel: string) => {
    setLocalProviders(prev => prev.map(p => {
      if (p.id === providerId) {
        const configs = parseToolSpecificConfigs(p);
        if (!configs[activeTab]) configs[activeTab] = {} as any;
        (configs[activeTab] as any).model_name = overrideModel;
        p.extra_config = serializeToolSpecificConfigs(p.extra_config, configs);
      }
      return p;
    }));
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      for (const p of localProviders) {
        const originalP = providers.find(o => o.id === p.id);
        if (!originalP) continue;
        if (originalP.tool_targets !== p.tool_targets || originalP.extra_config !== p.extra_config) {
          await api.providers.update(p);
        }
      }
      const orderedIds = localProviders.map(p => p.id);
      await api.providers.updateOrder(orderedIds);
      await fetch();
      alert("✅ 配置已成功下发同步！");
    } catch (e) {
      alert("保存失败：" + String(e));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="tool-sync-page animate-fade-in" style={{ padding: 24, height: "100%", display: "flex", flexDirection: "column", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 28, fontWeight: 700, margin: "0 0 8px 0" }}>终端配置同步控台</h1>
          <p style={{ color: "var(--color-text-secondary)", margin: 0, fontSize: 13 }}>
            选择您的 AI 编程终端工具，一键劫持并为其注入本地灾备网络与聚合模型配置。
          </p>
        </div>
        <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
          <label 
            title="开启后，下发到工具的地址将全被重写为 127.0.0.1 代理端"
            style={{ 
              display: "flex", alignItems: "center", gap: 6, cursor: "pointer", 
              fontSize: 13, 
              color: proxyTakeover ? "var(--color-primary)" : "var(--color-text-secondary)", 
              padding: "6px 12px", borderRadius: 8, 
              background: proxyTakeover ? "var(--color-primary-alpha)" : "var(--color-surface)", 
              transition: "all 0.2s",
              border: "1px solid var(--color-border)"
            }}
          >
            <input 
              type="checkbox" 
              checked={proxyTakeover} 
              onChange={e => {
                setProxyTakeover(e.target.checked);
                if (e.target.checked) alert("✅ 已开启代理截流模式！\n下发给终端工具的 API Endpoint 将自动转换成本机 127.0.0.1 代理。\n\n如首选节点宕机，网关将静默跨越至 P2 节点。");
              }} 
              style={{ accentColor: "var(--color-primary)" }}
            />
            🛡️ 故障转移接管代理
          </label>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
        
        {/* 顶部横向 Tabs 区域 */}
        <div style={{ 
          borderBottom: "1px solid var(--color-border)", 
          paddingBottom: 16, 
          marginBottom: 24, 
          flexShrink: 0, 
          overflowX: "auto" 
        }}>
          <div style={{ fontSize: 13, fontWeight: "bold", color: "var(--color-text-secondary)", marginBottom: 12 }}>
            支持注入的终端环境
          </div>
          <div style={{ display: "flex", flexDirection: "row", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
            {ALL_TOOLS.map(tool => {
              const matchedCount = localProviders.filter(p => parseToolTargets(p).includes(tool)).length;
              return (
                <button
                  key={tool}
                  style={{
                    display: "flex", alignItems: "center", gap: 10,
                    padding: "8px 16px", borderRadius: 20, border: "none",
                    background: activeTab === tool ? "var(--color-primary-alpha)" : "var(--color-surface)",
                    color: activeTab === tool ? "var(--color-primary)" : "var(--color-text-secondary)",
                    cursor: "pointer", transition: "all 0.2s",
                    fontWeight: activeTab === tool ? 600 : 400,
                    whiteSpace: "nowrap"
                  }}
                  onClick={() => setActiveTab(tool)}
                >
                  <span style={{ fontSize: 16 }}>{TOOL_ICONS[tool]}</span>
                  <span>{TOOL_TARGET_LABELS[tool]}</span>
                  {matchedCount > 0 && (
                    <span style={{ 
                      background: activeTab === tool ? "var(--color-primary)" : "var(--color-surface-raised)", 
                      color: activeTab === tool ? "#fff" : "inherit",
                      fontSize: 11, padding: "2px 6px", borderRadius: 12 
                    }}>
                      {matchedCount}
                    </span>
                  )}
                </button>
              )
            })}
          </div>
        </div>

        {/* 下方 面板内容 */}
        <div style={{ flex: 1, paddingRight: 4, overflowY: "auto" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
            <div>
              <h2 style={{ margin: "0 0 4px 0", display: "flex", alignItems: "center", gap: 12 }}>
                配置注入: {TOOL_TARGET_LABELS[activeTab]}
              </h2>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", fontFamily: "monospace" }}>
                文件锚点: {TOOL_TARGET_CONFIG_PATH[activeTab]} 
                {proxyTakeover && <span style={{ color: "var(--color-primary)", marginLeft: 8 }}>🛡️ [全局代理接管开启 中]</span>}
              </div>
            </div>
            <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
              <button className="btn btn-ghost" onClick={() => setShowProviderModal(true)}>
                ＋ 新增配置源
              </button>
              <button className="btn btn-primary" onClick={handleSave} disabled={isSaving}>
                {isSaving ? "应用中..." : "立刻下发覆盖"}
              </button>
            </div>
          </div>

          <div className="form-section-title" style={{ marginTop: 24 }}>灾备队列与模型指定 (拖拽手柄排序)</div>
          <div style={{ border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", background: "var(--color-surface)" }}>
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
              <SortableContext items={localProviders.map(p => p.id)} strategy={verticalListSortingStrategy}>
                {localProviders.map((p) => {
                  const isActive = parseToolTargets(p).includes(activeTab);
                  const extra = parseToolSpecificConfigs(p)[activeTab];
                  return (
                    <SortableProviderRow
                      key={p.id}
                      provider={p}
                      index={activeProviders.findIndex(a => a.id === p.id)}
                      isActive={isActive}
                      onToggle={(checked) => handleToggle(p.id, checked)}
                      localConfig={extra}
                      onChangeModel={(val) => handleChangeOverrideModel(p.id, val)}
                    />
                  );
                })}
              </SortableContext>
            </DndContext>
            {localProviders.length === 0 && (
              <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-tertiary)" }}>
                系统内暂无任何网关节点池。请点击右上角「＋ 新增配置源」添加。
              </div>
            )}
          </div>

          {activeProviders.length > 0 && (
            <div style={{ marginTop: 32 }}>
              <div className="form-section-title">配置文件模拟出流</div>
              <ConfigPreview
                baseUrl={proxyTakeover ? "http://127.0.0.1:23333/v1" : (activeProviders[0].base_url || "https://api.anthropic.com")}
                apiKey={activeProviders[0].api_key_id ? "sk-ais-xxxxx" : ""}
                toolTargets={[activeTab]}
                toolConfigs={activeProviders.reduce((acc, p) => {
                   acc[activeTab] = parseToolSpecificConfigs(p)[activeTab];
                   return acc;
                }, {} as any)}
              />
            </div>
          )}
        </div>
      </div>
      
      {showProviderModal && (
        <ProviderModal
          fixedTool={activeTab}
          onClose={() => setShowProviderModal(false)}
          onSuccess={() => {
            setShowProviderModal(false);
            fetch();
          }}
        />
      )}
    </div>
  );
}
