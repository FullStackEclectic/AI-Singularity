import { useEffect, useState, useMemo } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useMcpStore } from "../../stores/mcpStore";
import type { McpServer } from "../../types";
import { MCP_PRESETS, MCP_CATEGORIES, type McpPreset } from "../../data/mcpPresets";
import "./McpPage.css";

export default function McpPage() {
  const qc = useQueryClient();
  const { servers, isLoading, fetch, toggle } = useMcpStore();
  const [showAdd, setShowAdd] = useState(false);

  useEffect(() => {
    fetch();
  }, [fetch]);

  return (
    <div className="mcp-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">MCP Server</h1>
          <p className="page-subtitle">
            管理统一的 Model Context Protocol 服务器，支持全局挂载给各大 AI 编程助手
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
          <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
            ＋ 添加 Server
          </button>
        </div>
      </div>

      <div className="mcp-body">
        {isLoading && servers.length === 0 ? (
          <div className="empty-state">
             <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
             <span>加载中...</span>
          </div>
        ) : servers.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">🔌</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>暂无 MCP Server</h3>
            <p>基于 MCP 架构扩展你的 AI 的工具集与上下文感知能力</p>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              ＋ 添加第一个 MCP Server
            </button>
          </div>
        ) : (
          <div className="mcp-list">
            {servers.map((s) => (
              <McpCard 
                key={s.id} 
                server={s} 
                onToggle={() => toggle(s.id, !s.is_active).then(() => {
                  qc.invalidateQueries({ queryKey: ["dashboard-stats"] });
                })} 
              />
            ))}
          </div>
        )}
      </div>

      {showAdd && (
        <AddMcpModal
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

function McpCard({ server, onToggle }: { server: McpServer; onToggle: () => void }) {
  let argsDisplay = "无";
  try {
    if (server.args) {
      argsDisplay = JSON.parse(server.args).join(" ");
    }
  } catch(e) {}

  return (
    <div className={`mcp-card card ${server.is_active ? 'active-mcp' : ''} animate-fade-in`}>
      <div className="mcp-card-header">
        <div className="mcp-card-info">
          <div className="mcp-icon">🔌</div>
          <div>
            <div className="mcp-name">{server.name}</div>
            <div className="mcp-command font-mono text-muted">{server.command} {argsDisplay !== "无" ? argsDisplay : ""}</div>
          </div>
        </div>
        <div className="mcp-actions">
           <label className="toggle-switch">
             <input 
               type="checkbox" 
               checked={server.is_active}
               onChange={onToggle}
             />
             <span className="slider"></span>
           </label>
           <span className={`badge ${server.is_active ? 'badge-success' : 'badge-muted'}`}>
             {server.is_active ? '已启用' : '已停用'}
           </span>
        </div>
      </div>
      
      {server.env && server.env !== "{}" && (
        <div className="mcp-env">
           <span className="text-muted" style={{ fontSize: 13, marginRight: 8 }}>环境变量:</span>
           <span className="font-mono" style={{ fontSize: 12 }}>{server.env}</span>
        </div>
      )}
    </div>
  );
}

function AddMcpModal({ onClose, onSuccess }: { onClose: () => void; onSuccess: () => void }) {
  const { add } = useMcpStore();
  const [form, setForm] = useState({
    name: "",
    command: "npx",
    args: "-y @modelcontextprotocol/server-everything",
    envKey: "",
    envVal: ""
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [showPresets, setShowPresets] = useState(false);

  const applyPreset = (preset: McpPreset) => {
    setForm({
      name: preset.name,
      command: preset.command,
      args: preset.args.join(" "),
      envKey: preset.id === "github" ? "GITHUB_PERSONAL_ACCESS_TOKEN" : 
              preset.id === "brave-search" ? "BRAVE_API_KEY" : "",
      envVal: ""
    });
    setShowPresets(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim() || !form.command.trim()) {
      setError("名称和运行命令不能为空");
      return;
    }
    
    setIsSubmitting(true);
    setError("");
    try {
      let argsJson = "[]";
      if (form.args.trim()) {
        const splitArgs = form.args.trim().split(" ").filter(Boolean);
        argsJson = JSON.stringify(splitArgs);
      }

      let envJson = "{}";
      if (form.envKey.trim() && form.envVal.trim()) {
        envJson = JSON.stringify({ [form.envKey.trim()]: form.envVal.trim() });
      }

      await add({
        id: "",
        name: form.name.trim(),
        command: form.command.trim(),
        args: argsJson,
        env: envJson,
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
      <div className="modal modal-lg" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>添加 MCP Server</h2>
          <div style={{ display: "flex", gap: "var(--space-2)" }}>
            <button className="btn btn-ghost" onClick={() => setShowPresets(true)}>
              ✨ 从预设选择
            </button>
            <button className="btn btn-icon" onClick={onClose}>✕</button>
          </div>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-row">
            <label className="form-label">服务名称 *</label>
            <input
              className="form-input"
              placeholder="例如：Everything MCP"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </div>

          <div className="form-row">
            <label className="form-label">运行指令 (Command) *</label>
            <input
              className="form-input font-mono"
              placeholder="例如：npx 或 node"
              value={form.command}
              onChange={(e) => setForm({ ...form, command: e.target.value })}
            />
          </div>

          <div className="form-row">
            <label className="form-label">参数 (Args)</label>
            <input
              className="form-input font-mono"
              placeholder="例如：-y @modelcontextprotocol/server-everything"
              value={form.args}
              onChange={(e) => setForm({ ...form, args: e.target.value })}
            />
          </div>

          <div className="form-row flex-row-2">
            <div>
              <label className="form-label">环境变量 Key (可选)</label>
              <input
                className="form-input font-mono"
                placeholder="例如：API_KEY"
                value={form.envKey}
                onChange={(e) => setForm({ ...form, envKey: e.target.value })}
              />
            </div>
            <div>
              <label className="form-label">环境变量 Value</label>
              <input
                className="form-input font-mono"
                type="password"
                placeholder="值"
                value={form.envVal}
                onChange={(e) => setForm({ ...form, envVal: e.target.value })}
              />
            </div>
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

      {showPresets && (
        <McpPresetModal 
          onClose={() => setShowPresets(false)} 
          onSelect={applyPreset} 
        />
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP Preset Modal
// ─────────────────────────────────────────────────────────────────────────────

function McpPresetModal({ onClose, onSelect }: { onClose: () => void; onSelect: (p: McpPreset) => void }) {
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<typeof MCP_CATEGORIES[number] | "All">("All");

  const filtered = useMemo(() => {
    return MCP_PRESETS.filter(p => {
      if (activeTab !== "All" && p.category !== activeTab) return false;
      if (search) {
        const query = search.toLowerCase();
        return p.name.toLowerCase().includes(query) || p.description.toLowerCase().includes(query);
      }
      return true;
    });
  }, [search, activeTab]);

  return (
    <div className="modal-overlay preset-overlay" onClick={onClose} style={{ zIndex: 1000 }}>
      <div className="modal preset-modal" onClick={e => e.stopPropagation()} style={{ width: 680, maxWidth: "90vw" }}>
        <div className="modal-header">
          <h3>选择 MCP Template</h3>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>
        
        <div className="preset-search-bar" style={{ padding: "0 var(--space-6)" }}>
          <input 
            type="text" 
            className="form-input" 
            placeholder="搜索 MCP ..." 
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
        </div>

        <div className="preset-tabs" style={{ padding: "var(--space-3) var(--space-6) 0", display: "flex", gap: "var(--space-2)", borderBottom: "1px solid var(--color-border)" }}>
          <button className={`tab-btn ${activeTab === "All" ? "active" : ""}`} onClick={() => setActiveTab("All")}>全部</button>
          {MCP_CATEGORIES.map(cat => (
             <button key={cat} className={`tab-btn ${activeTab === cat ? "active" : ""}`} onClick={() => setActiveTab(cat)}>
               {cat}
             </button>
          ))}
        </div>

        <div className="preset-grid" style={{ padding: "var(--space-6)", display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", maxHeight: "60vh", overflowY: "auto" }}>
          {filtered.length === 0 ? (
            <div className="text-muted" style={{ gridColumn: "1 / -1", textAlign: "center", padding: "var(--space-8) 0" }}>没有找到相关的 MCP 预设</div>
          ) : (
            filtered.map(p => (
              <div key={p.id} className="preset-card card" onClick={() => onSelect(p)} style={{ cursor: "pointer", display: "flex", flexDirection: "column", gap: 8 }}>
                 <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                   <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                     <span style={{ fontSize: 20 }}>{p.icon}</span>
                     <span style={{ fontWeight: 600, fontSize: 14 }}>{p.name}</span>
                   </div>
                   {p.recommended && <span style={{ fontSize: 10, background: "var(--color-success)", color: "#000", padding: "2px 6px", borderRadius: 4, fontWeight: "bold" }}>推荐</span>}
                 </div>
                 <p className="text-muted" style={{ fontSize: 12, lineHeight: 1.4, flex: 1 }}>{p.description}</p>
                 <div style={{ fontSize: 11, background: "var(--bg-inset)", padding: "4px 6px", borderRadius: 4, fontFamily: "monospace", color: "var(--color-accent)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                   {p.command} {p.args.join(" ")}
                 </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
