import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, AiTool, Platform } from "../../types";
import { PLATFORM_LABELS, AI_TOOL_LABELS } from "../../types";
import "./ProvidersPage.css";

const AI_TOOLS: { value: AiTool; label: string; icon: string }[] = [
  { value: "claude_code", label: "Claude Code", icon: "🤖" },
  { value: "aider", label: "Aider", icon: "💻" },
  { value: "codex", label: "Codex", icon: "🧠" },
  { value: "gemini_cli", label: "Gemini CLI", icon: "✨" },
  { value: "open_code", label: "OpenCode", icon: "🌐" },
];

const PLATFORMS: { value: Platform; label: string }[] = [
  { value: "open_ai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic (Claude)" },
  { value: "gemini", label: "Google Gemini" },
  { value: "deep_seek", label: "DeepSeek" },
  { value: "aliyun", label: "阿里云百炼" },
  { value: "bytedance", label: "字节豆包" },
  { value: "moonshot", label: "Moonshot (Kimi)" },
  { value: "zhipu", label: "智谱 GLM" },
  { value: "custom", label: "自定义接口" },
];

export default function ProvidersPage() {
  const qc = useQueryClient();
  const { providers, isLoading, fetch, switchProvider } = useProviderStore();
  const [showAdd, setShowAdd] = useState(false);

  useEffect(() => {
    fetch();
  }, [fetch]);

  return (
    <div className="providers-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">AI 工具接入层</h1>
          <p className="page-subtitle">
            一键将你的 API 切换到常用的 AI 编程工具中
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
          <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
            ＋ 添加配置
          </button>
        </div>
      </div>

      <div className="providers-body">
        {isLoading && providers.length === 0 ? (
          <div className="empty-state">
             <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
             <span>加载中...</span>
          </div>
        ) : providers.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">⚡</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>暂无接入配置</h3>
            <p>点击「添加配置」为你的 AI 工具绑定大模型服务</p>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              ＋ 添加第一个配置
            </button>
          </div>
        ) : (
          <div className="providers-grid">
            {providers.map((p) => (
              <ProviderCard 
                key={p.id} 
                provider={p} 
                onToggle={() => switchProvider(p.id, p.ai_tool).then(() => {
                  qc.invalidateQueries({ queryKey: ["dashboard-stats"] });
                })} 
              />
            ))}
          </div>
        )}
      </div>

      {showAdd && (
        <AddProviderModal
          onClose={() => setShowAdd(false)}
          onSuccess={() => {
            setShowAdd(false);
            qc.invalidateQueries({ queryKey: ["dashboard-stats"] });
          }}
        />
      )}
    </div>
  );
}

function ProviderCard({ provider, onToggle }: { provider: ProviderConfig; onToggle: () => void }) {
  const toolIcon = AI_TOOLS.find(t => t.value === provider.ai_tool)?.icon || "🤖";
  
  return (
    <div className={`card provider-card ${provider.is_active ? 'active-card' : ''} animate-fade-in`}>
       <div className="provider-header">
         <div className="provider-title">
           <span className="tool-icon">{toolIcon}</span>
           <div>
             <h3>{AI_TOOL_LABELS[provider.ai_tool]}</h3>
             <div className="text-muted" style={{ fontSize: 13 }}>{provider.name}</div>
           </div>
         </div>
         <label className="toggle-switch">
           <input 
             type="checkbox" 
             checked={provider.is_active}
             onChange={onToggle}
           />
           <span className="slider"></span>
         </label>
       </div>
       <div className="provider-details">
         <div className="detail-row">
           <span className="text-muted">平台:</span>
           <span>{PLATFORM_LABELS[provider.platform]}</span>
         </div>
         <div className="detail-row">
           <span className="text-muted">模型:</span>
           <span className="font-mono">{provider.model_name}</span>
         </div>
         {provider.base_url && (
           <div className="detail-row">
             <span className="text-muted">代理地址:</span>
             <span className="font-mono text-break truncate-text" title={provider.base_url}>{provider.base_url}</span>
           </div>
         )}
       </div>
    </div>
  );
}

function AddProviderModal({ onClose, onSuccess }: { onClose: () => void; onSuccess: () => void }) {
  const { add } = useProviderStore();
  const [form, setForm] = useState({
    name: "",
    ai_tool: "claude_code" as AiTool,
    platform: "open_ai" as Platform,
    model_name: "gpt-4-turbo",
    base_url: "",
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim() || !form.model_name.trim()) {
      setError("名称和模型不能为空");
      return;
    }
    setIsSubmitting(true);
    setError("");
    try {
      await add({
        id: "",
        name: form.name.trim(),
        ai_tool: form.ai_tool,
        platform: form.platform,
        model_name: form.model_name.trim(),
        base_url: form.base_url.trim() || undefined,
        is_active: false,
        created_at: "",
        updated_at: ""
      });
      onSuccess();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>添加 Provider 配置</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-row">
            <label className="form-label">标识名称 *</label>
            <input
              className="form-input"
              placeholder="例如：Claude 面向开发"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </div>

          <div className="form-row">
            <label className="form-label">目标工具 *</label>
            <select
              className="form-input"
              value={form.ai_tool}
              onChange={(e) => setForm({ ...form, ai_tool: e.target.value as AiTool })}
            >
              {AI_TOOLS.map((t) => (
                <option key={t.value} value={t.value}>{t.icon} {t.label}</option>
              ))}
            </select>
            <p className="form-hint">选择你要一键注入环境的 AI 编程工具</p>
          </div>

          <div className="form-row flex-row-2">
            <div>
              <label className="form-label">接口平台 *</label>
              <select
                className="form-input"
                value={form.platform}
                onChange={(e) => setForm({ ...form, platform: e.target.value as Platform })}
              >
                {PLATFORMS.map((p) => (
                  <option key={p.value} value={p.value}>{p.label}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="form-label">默认模型 *</label>
              <input
                className="form-input font-mono"
                placeholder="例如：gpt-4o"
                value={form.model_name}
                onChange={(e) => setForm({ ...form, model_name: e.target.value })}
              />
            </div>
          </div>

          <div className="form-row">
             <label className="form-label">代理地址 (可选)</label>
             <input
               className="form-input"
               placeholder="http://127.0.0.1:3000/v1"
               value={form.base_url}
               onChange={(e) => setForm({ ...form, base_url: e.target.value })}
             />
             <p className="form-hint">留空将使用当前内置网关或平台默认地址</p>
          </div>

          {error && <div className="form-error">{error}</div>}

          <div className="modal-footer">
            <button type="button" className="btn btn-ghost" onClick={onClose}>取消</button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isSubmitting}
            >
              {isSubmitting ? "保存中..." : "保存配置"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
