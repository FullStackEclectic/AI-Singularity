import { useState, useEffect, useMemo } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ToolTarget, ProviderConfig } from "../../types";
import { TOOL_TARGET_LABELS, TOOL_TARGET_CONFIG_PATH, parseToolTargets, parseToolSpecificConfigs, serializeToolSpecificConfigs } from "../../types";
import ConfigPreview from "./ConfigPreview";
import { api } from "../../lib/api";
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
import "./ProviderModal.css";

function getToolModelOverride(localConfig: any): string {
  if (!localConfig || typeof localConfig !== "object") return "";
  if (typeof localConfig.model === "string") return localConfig.model;
  if (typeof localConfig.model_name === "string") return localConfig.model_name;
  return "";
}

function getToolDefaultBaseUrl(tool: ToolTarget): string {
  switch (tool) {
    case "claude_code":
      return "https://api.anthropic.com";
    case "gemini_cli":
      return "https://generativelanguage.googleapis.com";
    case "codex":
    case "open_code":
    case "open_claw":
      return "https://api.openai.com/v1";
    case "aider":
      return "";
    default:
      return "";
  }
}

// --- 可拖拽的备选源条目 ---
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
            <label style={{ fontSize: 10, color: "var(--color-text-tertiary)" }}>单独指定模型（留空使用默认）</label>
            <input 
              className="form-input"
              style={{ padding: "4px 8px", fontSize: 12 }}
              placeholder={provider.model_name || "自定义模型..."}
              value={getToolModelOverride(localConfig)}
              onChange={(e) => onChangeModel(e.target.value)}
              onClick={e => e.stopPropagation()}
            />
          </div>
        )}
      </div>
    </div>
  );
}

export default function ToolConfigModal({ 
  tool, 
  onClose,
  proxyTakeover
}: { 
  tool: ToolTarget; 
  onClose: () => void;
  proxyTakeover: boolean;
}) {
  const { providers, fetch } = useProviderStore();
  const [localProviders, setLocalProviders] = useState<ProviderConfig[]>([]);
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    setLocalProviders([...providers].sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0)));
  }, [providers]);

  const activeProviders = useMemo(() => {
    return localProviders.filter(p => parseToolTargets(p).includes(tool));
  }, [localProviders, tool]);

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
        if (checked && !targets.includes(tool)) targets.push(tool);
        if (!checked) targets = targets.filter(t => t !== tool);
        p.tool_targets = JSON.stringify(targets);
      }
      return p;
    }));
  };

  const handleChangeOverrideModel = (providerId: string, overrideModel: string) => {
    setLocalProviders(prev => prev.map(p => {
      if (p.id === providerId) {
        const configs = parseToolSpecificConfigs(p);
        if (!configs[tool]) configs[tool] = {} as any;
        if (overrideModel.trim()) {
          (configs[tool] as any).model = overrideModel;
        } else if ((configs[tool] as any)?.model) {
          delete (configs[tool] as any).model;
        }
        if ((configs[tool] as any)?.model_name) {
          delete (configs[tool] as any).model_name;
        }
        p.extra_config = serializeToolSpecificConfigs(p.extra_config, configs);
      }
      return p;
    }));
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // 1. 更新每个 Provider 的修改（包含 tool_targets 的增删和 override model）
      for (const p of localProviders) {
        const originalP = providers.find(o => o.id === p.id);
        if (!originalP) continue;
        if (originalP.tool_targets !== p.tool_targets || originalP.extra_config !== p.extra_config) {
          await api.providers.update(p);
        }
      }
      // 2. 更新顺序
      const orderedIds = localProviders.map(p => p.id);
      await api.providers.updateOrder(orderedIds);
      
      await fetch();
      onClose();
    } catch (e) {
      setMessage("保存失败：" + String(e));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()} style={{ minWidth: 600, maxWidth: 800 }}>
        <div className="modal-header">
          <div>
            <h2 style={{ display: "flex", alignItems: "center", gap: 12 }}>
              配置目标：{TOOL_TARGET_LABELS[tool]}
            </h2>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", fontFamily: "monospace", marginTop: 4 }}>
              配置文件路径：{TOOL_TARGET_CONFIG_PATH[tool]} 
              {proxyTakeover && <span style={{ color: "var(--color-primary)", marginLeft: 8 }}>🛡️ [全局代理接管开启 中]</span>}
            </div>
          </div>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        <div className="modal-body">
          {message && (
            <div className="alert alert-info" style={{ marginBottom: 12 }}>
              {message}
            </div>
          )}
          <div className="form-section-title">配置源列表（支持拖拽排序）</div>
          <p className="form-hint" style={{ marginTop: -8, marginBottom: 12 }}>
            勾选要绑定给该工具的配置源，排在最前的将优先使用。
          </p>
          
          <div style={{ border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", background: "var(--color-surface)" }}>
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
              <SortableContext items={localProviders.map(p => p.id)} strategy={verticalListSortingStrategy}>
                {localProviders.map((p) => {
                  const isActive = parseToolTargets(p).includes(tool);
                  const extra = parseToolSpecificConfigs(p)[tool];
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
                暂无可用配置源，请先添加。
              </div>
            )}
          </div>

          {activeProviders.length > 0 && (
            <div style={{ marginTop: 24 }}>
              <div className="form-section-title">配置文件预览</div>
              <ConfigPreview
                baseUrl={proxyTakeover ? "http://127.0.0.1:23333/v1" : (activeProviders[0].base_url || getToolDefaultBaseUrl(tool))}
                apiKey={activeProviders[0].api_key_id ? "sk-ais-xxxxx" : ""}
                provider={activeProviders[0]}
                toolTargets={[tool]}
                toolConfigs={{
                  [tool]: parseToolSpecificConfigs(activeProviders[0])[tool],
                }}
              />
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose}>取消</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? "应用中..." : "保存并应用"}
          </button>
        </div>
      </div>
    </div>
  );
}
