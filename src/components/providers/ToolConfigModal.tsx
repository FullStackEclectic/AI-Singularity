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
            <label style={{ fontSize: 10, color: "var(--color-text-tertiary)" }}>独立劫持模型定义 (留空使用默认)</label>
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
        (configs[tool] as any).model_name = overrideModel;
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
      alert("保存失败：" + String(e));
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
              配置注入: {TOOL_TARGET_LABELS[tool]}
            </h2>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", fontFamily: "monospace", marginTop: 4 }}>
              配置文件锚点: {TOOL_TARGET_CONFIG_PATH[tool]} 
              {proxyTakeover && <span style={{ color: "var(--color-primary)", marginLeft: 8 }}>🛡️ [全局代理接管开启 中]</span>}
            </div>
          </div>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        <div className="modal-body">
          <div className="form-section-title">灾备队列与节点池 (支持拖拽排序)</div>
          <p className="form-hint" style={{ marginTop: -8, marginBottom: 12 }}>
            勾选你想绑定给该工具的节点。排在最前（P1）的节点将作为代理首发源。
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
                系统内暂无可用网关节点，请先前往高级设置添加。
              </div>
            )}
          </div>

          {activeProviders.length > 0 && (
            <div style={{ marginTop: 24 }}>
              <div className="form-section-title">底层配置文件实时预览</div>
              <ConfigPreview
                baseUrl={proxyTakeover ? "http://127.0.0.1:23333/v1" : (activeProviders[0].base_url || "https://api.anthropic.com")}
                apiKey={activeProviders[0].api_key_id ? "sk-ais-xxxxx" : ""}
                toolTargets={[tool]}
                toolConfigs={activeProviders.reduce((acc, p) => {
                   acc[tool] = parseToolSpecificConfigs(p)[tool];
                   return acc;
                }, {} as any)}
              />
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose}>取消</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? "应用中..." : "立刻下发覆盖"}
          </button>
        </div>
      </div>
    </div>
  );
}
