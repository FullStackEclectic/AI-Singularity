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
      <div className="info-banner" style={{ background: "var(--color-bg-secondary)", marginBottom: 32 }}>
        <div className="proxy-header-info">
          <h1 className="proxy-title">
            <Server size={24} className="text-accent" />
            本地代理网关
          </h1>
          <p style={{ margin: 0, fontSize: 13, color: "var(--color-text-secondary)" }}>
            负责本地请求转发与协议适配，支持 OpenAI / Anthropic / Gemini 三种格式互转。
          </p>
        </div>
        <div className="proxy-action-panel">
          <div className="port-config" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, color: "var(--color-text-secondary)", marginRight: 16 }}>
            <span>监听端口</span>
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
            className={`btn ${isRunning ? "btn-ghost" : "btn-primary"}`}
            onClick={() => startMut.mutate()}
            disabled={startMut.isPending || isRunning}
            style={{ minWidth: 120 }}
          >
            {isRunning ? <><Check size={16}/> 运行中</> : startMut.isPending ? "启动中..." : <><Zap size={16}/> 启动代理</>}
          </button>
          <div className={`proxy-status-pill ${isRunning ? "online" : "offline"}`}>
            <span className="status-dot" />
            {isRunning ? "运行中" : "已停止"}
          </div>
        </div>
      </div>

      <CloudflaredTunnelPanel localPort={port} />

      <div className="proxy-grid">
        {/* 连接状态 */}
        <div className="proxy-card">
          <div className="proxy-card-header">
            <Activity size={18} className="proxy-card-header-icon" />
            连接状态
          </div>
          
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div className="info-banner" style={{ margin: 0, padding: "12px 16px", background: "var(--color-success-dim)" }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>本地服务地址</div>
                <div style={{ fontSize: 14, fontFamily: "monospace", color: "var(--color-text-primary)", fontWeight: 600, marginTop: 4 }}>
                  {endpoint}
                </div>
              </div>
              <button className="btn btn-ghost btn-sm" onClick={handleCopy}>
                {copied ? "已复制" : "复制"}
              </button>
            </div>

            <div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">可用账号数</span>
                <span className="proxy-stat-value">{validNodes} 个活跃</span>
              </div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">请求拦截模式</span>
                <span className="proxy-stat-value">流式 SSE</span>
              </div>
              <div className="proxy-stat-row">
                <span className="proxy-stat-label">多模态支持</span>
                <span className="proxy-stat-value" style={{ color: "var(--color-success)" }}>已启用（Imagen-3）</span>
              </div>
            </div>
          </div>
        </div>

        {/* 协议转换 */}
        <div className="proxy-card">
          <div className="proxy-card-header">
            <Network size={18} className="proxy-card-header-icon" />
            协议转换
          </div>
          <div className="protocol-list">
            {[
              { flow: "OpenAI ↔ Anthropic", status: "ONLINE", type: "流式转换" },
              { flow: "OpenAI ↔ Gemini", status: "ONLINE", type: "原生映射" },
              { flow: "OpenAI ↔ DeepSeek", status: "ONLINE", type: "直通" },
              { flow: "Auth0IDE ↔ ClaudeCode", status: "STANDBY", type: "账号注入" }
            ].map((p, i) => (
              <div key={i} className="protocol-item">
                <div className="protocol-flow">{p.flow}</div>
                <div className="protocol-meta">
                  <span className="protocol-tag">{p.type}</span>
                  <span className={`protocol-status ${p.status === 'ONLINE' ? 'online' : 'standby'}`}>
                    {p.status === 'ONLINE' ? '运行中' : '待机'}
                  </span>
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
          模型映射规则
        </div>
        <p style={{ margin: "0 0 16px 0", fontSize: 13, color: "var(--color-text-secondary)" }}>
          设置模型映射规则：当客户端请求某个模型时，自动转发至目标模型提供商。
        </p>
        <ModelMappingPanel />
      </div>

      <div className="proxy-card" style={{ marginTop: 24 }}>
        <div className="proxy-card-header">
          <ShieldCheck size={18} className="proxy-card-header-icon" />
          连通性测试
        </div>
        <div style={{ height: 380 }}>
          <PlaygroundPanel />
        </div>
      </div>
    </div>
  );
}
