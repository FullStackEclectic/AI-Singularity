import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Server, Activity, Network, Zap, Settings, ShieldCheck, Check } from "lucide-react";
import type { IdeAccount } from "../../types";
import PlaygroundPanel from "./PlaygroundPanel";
import ModelMappingPanel from "./ModelMappingPanel";
import CloudflaredTunnelPanel from "./CloudflaredTunnelPanel";
import ProxyEngineSettingsPanel from "./ProxyEngineSettingsPanel";
import "./ProxyPage.css";

interface ProxyStatus {
  running: boolean;
  port: number;
  endpoint: string;
}

export default function ProxyPage() {
  const qc = useQueryClient();
  const [port, setPort] = useState(8765);
  const [copied, setCopied] = useState(false);

  const { data: status } = useQuery<ProxyStatus>({
    queryKey: ["proxy-status"],
    queryFn: () => invoke("get_proxy_status", { port }),
    refetchInterval: 3000,
  });

  const { data: accounts = [] } = useQuery<IdeAccount[]>({
    queryKey: ["ide-accounts"],
    queryFn: () => invoke("get_all_ide_accounts"),
  });

  const startMut = useMutation({
    mutationFn: () => invoke<ProxyStatus>("start_proxy", { port }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["proxy-status"] }),
  });

  const isRunning = status?.running ?? false;
  const endpoint = status?.endpoint ?? `http://127.0.0.1:${port}/v1`;
  const validNodes = accounts.filter(a => a.status === 'active').length;

  const handleCopy = () => {
    navigator.clipboard.writeText(endpoint);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="proxy-page">
      <div className="info-banner" style={{ background: "var(--color-surface)", marginBottom: 32 }}>
        <div className="proxy-header-info">
          <h1 className="proxy-title">
            <Server size={24} className="text-primary" />
            Proxy Gateway 总控制台引擎
          </h1>
          <p style={{ margin: 0, fontSize: 13, color: "var(--color-text-secondary)" }}>
            承载协议转录拦截、流级劫持与本地端口投射的核心组件。
          </p>
        </div>
        <div className="proxy-action-panel">
          <div className="port-config" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, color: "var(--color-text-secondary)", marginRight: 16 }}>
            <span>端口:</span>
            <input
              type="number"
              className="form-input"
              style={{ width: 80, padding: "4px 8px" }}
              value={port}
              onChange={(e) => setPort(Number(e.target.value))}
              disabled={isRunning}
            />
          </div>
          <button
            className={`btn ${isRunning ? "btn-secondary" : "btn-primary"}`}
            onClick={() => startMut.mutate()}
            disabled={startMut.isPending || isRunning}
            style={{ minWidth: 140 }}
          >
            {isRunning ? <><Check size={16}/> 引擎运转中</> : startMut.isPending ? "启动中..." : <><Zap size={16}/> 启动本地代理</>}
          </button>
          <div className={`proxy-status-pill ${isRunning ? "online" : "offline"}`}>
            <span className="status-dot" />
            {isRunning ? "ONLINE" : "OFFLINE"}
          </div>
        </div>
      </div>

      <CloudflaredTunnelPanel localPort={port} />

      <div className="proxy-grid">
        {/* Core Settings / Dashboard */}
        <div className="proxy-card">
          <div className="proxy-card-header">
            <Activity size={18} className="proxy-card-header-icon" />
            会话雷达监控 (Session Radar)
          </div>
          
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div className="info-banner" style={{ margin: 0, padding: "12px 16px", background: "rgba(16, 185, 129, 0.04)" }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>本地服务入口</div>
                <div style={{ fontSize: 14, fontFamily: "monospace", color: "var(--color-text)", fontWeight: 600, marginTop: 4 }}>
                  {endpoint}
                </div>
              </div>
              <button className="btn btn-secondary btn-sm" onClick={handleCopy}>
                {copied ? "已复制" : "复制"}
              </button>
            </div>

            <div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">可用池化凭据源</span>
                <span className="proxy-stat-value">{validNodes} 活跃指纹</span>
              </div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">流体劫持模式 (Intercept)</span>
                <span className="proxy-stat-value">Deep Stream / SSE</span>
              </div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">跨模态支持</span>
                <span className="proxy-stat-value" style={{ color: "#10B981" }}>启用中 (Imagen-3)</span>
              </div>
            </div>
          </div>
        </div>

        {/* Protocols */}
        <div className="proxy-card">
          <div className="proxy-card-header">
            <Network size={18} className="proxy-card-header-icon" />
            运行时协议栈 (Transmutation)
          </div>
          <div className="protocol-list">
            {[
              { flow: "OpenAI ↔ Anthropic", status: "ONLINE", type: "SSE_HIJACK" },
              { flow: "OpenAI ↔ Gemini", status: "ONLINE", type: "NATIVE_MAP" },
              { flow: "OpenAI ↔ DeepSeek", status: "ONLINE", type: "PASSTHROUGH" },
              { flow: "Auth0IDE ↔ ClaudeCode", status: "STANDBY", type: "FORGE_ID" }
            ].map((p, i) => (
              <div key={i} className="protocol-item">
                <div className="protocol-flow">{p.flow}</div>
                <div className="protocol-meta">
                  <span className="protocol-tag">{p.type}</span>
                  <span className={`protocol-status ${p.status === 'ONLINE' ? 'online' : 'standby'}`}>{p.status}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <ProxyEngineSettingsPanel />

      <div className="proxy-card" style={{ marginTop: 24 }}>
        <div className="proxy-card-header">
          <Settings size={18} className="proxy-card-header-icon" />
          全量自动模型重写 (Model Mapping Rules)
        </div>
        <p style={{ margin: "0 0 16px 0", fontSize: 13, color: "var(--color-text-secondary)" }}>
          设定模型降维与路由映射规则：当客户端使用来源模型请求网关时，透明转写至目标大模型提供商。
        </p>
        <ModelMappingPanel />
      </div>

      <div className="proxy-card" style={{ marginTop: 24 }}>
        <div className="proxy-card-header">
          <ShieldCheck size={18} className="proxy-card-header-icon" />
          沙盒内侧连通性验证 (Sandbox)
        </div>
        <div style={{ height: 380 }}>
          <PlaygroundPanel />
        </div>
      </div>
    </div>
  );
}
