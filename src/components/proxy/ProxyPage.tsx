import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
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

  const startMut = useMutation({
    mutationFn: () => invoke<ProxyStatus>("start_proxy", { port }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["proxy-status"] }),
  });

  const isRunning = status?.running ?? false;
  const endpoint = status?.endpoint ?? `http://127.0.0.1:${port}/v1`;

  const handleCopy = () => {
    navigator.clipboard.writeText(endpoint);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="proxy-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">本地代理网关</h1>
          <p className="page-subtitle">OpenAI 兼容接口 · 三协议互转 · 配额感知路由</p>
        </div>
        <div className="proxy-status-indicator">
          <span className={`status-dot ${isRunning ? "valid" : "unknown"}`} />
          <span className="text-secondary" style={{ fontSize: 13 }}>
            {isRunning ? "运行中" : "已停止"}
          </span>
        </div>
      </div>

      <div className="proxy-body">
        {/* 控制卡片 */}
        <div className="card proxy-control-card">
          <div className="proxy-control-header">
            <div className="proxy-control-info">
              <div className="proxy-control-title">代理状态</div>
              <div className="proxy-control-sub text-muted">
                AI Singularity 在本地启动 OpenAI 兼容代理，将请求自动路由到最优账号
              </div>
            </div>
            <button
              className={`btn ${isRunning ? "btn-ghost" : "btn-primary"} proxy-toggle-btn`}
              onClick={() => startMut.mutate()}
              disabled={startMut.isPending || isRunning}
            >
              {isRunning ? "● 运行中" : startMut.isPending ? "启动中..." : "▶ 启动代理"}
            </button>
          </div>

          {/* 端点地址 */}
          <div className="proxy-endpoint">
            <div className="proxy-endpoint-label text-muted">接口地址</div>
            <div className="proxy-endpoint-row">
              <code className="proxy-endpoint-url font-mono">{endpoint}</code>
              <button className="btn btn-ghost btn-sm" onClick={handleCopy}>
                {copied ? "✓ 已复制" : "复制"}
              </button>
            </div>
          </div>

          {/* 端口设置 */}
          <div className="proxy-port-row">
            <label className="text-muted" style={{ fontSize: 13 }}>端口</label>
            <input
              type="number"
              className="proxy-port-input"
              value={port}
              min={1024}
              max={65535}
              disabled={isRunning}
              onChange={(e) => setPort(Number(e.target.value))}
            />
          </div>
        </div>

        {/* 如何使用 */}
        <div className="card" style={{ marginTop: "var(--space-5)" }}>
          <h2 className="section-title">如何使用</h2>
          <div className="proxy-usage-grid">
            <div className="proxy-usage-item">
              <div className="proxy-usage-icon">🔧</div>
              <div className="proxy-usage-title">通用配置</div>
              <code className="proxy-usage-code">
                OPENAI_BASE_URL={endpoint}
              </code>
            </div>
            <div className="proxy-usage-item">
              <div className="proxy-usage-icon">🤖</div>
              <div className="proxy-usage-title">Claude Code</div>
              <code className="proxy-usage-code">
                {"claude --openai-base-url " + endpoint}
              </code>
            </div>
            <div className="proxy-usage-item">
              <div className="proxy-usage-icon">⚡</div>
              <div className="proxy-usage-title">Aider</div>
              <code className="proxy-usage-code">
                {"--openai-api-base " + endpoint}
              </code>
            </div>
          </div>
        </div>

        {/* 支持的协议 */}
        <div className="card" style={{ marginTop: "var(--space-5)" }}>
          <h2 className="section-title">协议转换支持</h2>
          <div className="protocol-list">
            {[
              { from: "OpenAI", to: "OpenAI", desc: "直接转发，无转换" },
              { from: "OpenAI", to: "Anthropic", desc: "自动转换请求/响应格式" },
              { from: "OpenAI", to: "Gemini", desc: "自动转换请求/响应格式" },
              { from: "OpenAI", to: "DeepSeek", desc: "直接转发（兼容接口）" },
              { from: "OpenAI", to: "百炼/豆包/Kimi", desc: "直接转发（兼容接口）" },
            ].map((p, i) => (
              <div key={i} className="protocol-item">
                <span className="badge badge-info">{p.from}</span>
                <span className="text-muted" style={{ fontSize: 16 }}>→</span>
                <span className="badge badge-success">{p.to}</span>
                <span className="text-muted" style={{ marginLeft: "auto", fontSize: 12 }}>{p.desc}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
