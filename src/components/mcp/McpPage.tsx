import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useMcpStore } from "../../stores/mcpStore";
import type { McpServer } from "../../types";
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
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>添加 MCP Server</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
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
    </div>
  );
}
