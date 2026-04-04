import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { IdeAccount } from "../../types";
import PlaygroundPanel from "./PlaygroundPanel";
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
    <div className="proxy-page cyberpunk-theme">
      <div className="page-header">
        <div>
          <h1 className="cyber-title">
            <span className="cyber-glitch" data-text="Proxy Gateway">PROXY GATEWAY</span>
            <span className="cyber-sub"> // 总控制台引擎</span>
          </h1>
          <p className="page-subtitle glow-text">协议转录拦截 · 流级欺诈网络 · 降维指纹伪装</p>
        </div>
        <div className="proxy-status-indicator">
          <div className={`radar-ping ${isRunning ? "active" : ""}`} />
          <span className="cyber-status-text" style={{ fontSize: 14 }}>
            {isRunning ? "[ 引擎运转中 ]" : "[ 引擎离线 ]"}
          </span>
        </div>
      </div>

      <div className="proxy-body">
        {/* Core Control */}
        <div className="cyber-card">
          <div className="cyber-card-header">
            <h2 className="cyber-section-title">CORE_CONTROL // 核心中枢</h2>
            <button
              className={`cyber-btn ${isRunning ? "active" : ""}`}
              onClick={() => startMut.mutate()}
              disabled={startMut.isPending || isRunning}
            >
              <span className="btn-icon">⚡</span>
              {isRunning ? "NETWORK ONLINE" : startMut.isPending ? "INITIALIZING..." : "ENGAGE PROXY"}
            </button>
          </div>

          <div className="cyber-endpoint-box">
            <div className="endpoint-label">LOCAL_ENDPOINT:</div>
            <div className="endpoint-code-row">
              <code className="endpoint-url">{endpoint}</code>
              <button className="cyber-icon-btn" onClick={handleCopy} title="复制地址">
                {copied ? "COPIED" : "COPY"}
              </button>
            </div>
            <div className="port-config">
              <span>PORT:</span>
              <input
                type="number"
                className="cyber-input-mini"
                value={port}
                onChange={(e) => setPort(Number(e.target.value))}
                disabled={isRunning}
              />
            </div>
          </div>
        </div>

        {/* Session Radar Grid */}
        <div className="cyber-grid">
          <div className="cyber-card">
            <h2 className="cyber-section-title">
              <span className="pulse-dot"></span> SESSION_RADAR // 会话雷达
            </h2>
            <div className="radar-monitor">
              <div className="radar-circle">
                <div className="radar-sweep"></div>
                {isRunning && <div className="radar-blip"></div>}
              </div>
              <div className="radar-stats">
                <div className="stat-line">
                  <span className="stat-label">GHOST_NODES_POOL:</span>
                  <span className="stat-val highlight">{validNodes} ACTIVE</span>
                </div>
                <div className="stat-line">
                  <span className="stat-label">INTERCEPT_MODE:</span>
                  <span className="stat-val">DEEP_STREAM (SSE)</span>
                </div>
                <div className="stat-line">
                  <span className="stat-label">IMAGE_BYPASS:</span>
                  <span className="stat-val text-success">ENABLED (IMAGEN-3)</span>
                </div>
              </div>
            </div>
          </div>

          <div className="cyber-card">
            <h2 className="cyber-section-title">TRANSMUTATION // 协议转录</h2>
            <div className="cyber-protocol-list">
              {[
                { flow: "OpenAI ↔ Anthropic", status: "ONLINE", type: "SSE_HIJACK" },
                { flow: "OpenAI ↔ Gemini", status: "ONLINE", type: "NATIVE_MAP" },
                { flow: "OpenAI ↔ DeepSeek", status: "ONLINE", type: "PASSTHROUGH" },
                { flow: "Auth0IDE ↔ ClaudeCode", status: "STANDBY", type: "FINGERPRINT_FORGE" }
              ].map((p, i) => (
                <div key={i} className="cyber-protocol-item">
                  <div className="protocol-flow">{p.flow}</div>
                  <div className="protocol-meta">
                    <span className="cyber-tag">{p.type}</span>
                    <span className={`status-badge ${p.status === 'ONLINE' ? 'online' : 'standby'}`}>{p.status}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
        {/* Playground */}
        <div className="cyber-card" style={{ marginTop: "var(--space-4)" }}>
          <h2 className="cyber-section-title">SIGNAL_TEST // API 连通性测试沙盒</h2>
          <div style={{ height: 380 }}>
            <PlaygroundPanel />
          </div>
        </div>
      </div>
    </div>
  );
}
