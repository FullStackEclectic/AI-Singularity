import { useState } from "react";
import { api } from "../../lib/api";
import "./SpeedTestPage.css";

interface SpeedTestResult {
  platform: string;
  endpoint: string;
  latency_ms: number | null;
  status: "ok" | "timeout" | "error";
}

const PLATFORM_LABELS: Record<string, string> = {
  open_ai:   "OpenAI",
  anthropic: "Anthropic",
  gemini:    "Google Gemini",
  deep_seek: "DeepSeek",
  aliyun:    "阿里云百炼",
  bytedance: "字节豆包",
  moonshot:  "Moonshot (Kimi)",
  zhipu:     "智谱 GLM",
  nvidia_nim:"NVIDIA NIM",
};

const PLATFORM_ICONS: Record<string, string> = {
  open_ai:   "◎",
  anthropic: "◈",
  gemini:    "◆",
  deep_seek: "◉",
  aliyun:    "◐",
  bytedance: "◑",
  moonshot:  "◒",
  zhipu:     "◓",
  nvidia_nim:"◔",
};

function getLatencyColor(ms: number | null, status: string): string {
  if (status !== "ok" || ms === null) return "var(--color-danger)";
  if (ms < 300) return "var(--color-success)";
  if (ms < 800) return "var(--color-warning)";
  return "var(--color-danger)";
}

function getLatencyLabel(ms: number | null, status: string): string {
  if (status === "timeout") return "超时";
  if (status === "error") return "错误";
  if (ms === null) return "—";
  return `${ms} ms`;
}

function LatencyBar({ ms, status }: { ms: number | null; status: string }) {
  const maxMs = 2000;
  const pct = ms ? Math.min((ms / maxMs) * 100, 100) : 100;
  const color = getLatencyColor(ms, status);
  return (
    <div className="latency-bar-track">
      <div
        className="latency-bar-fill"
        style={{ width: `${status === "ok" && ms ? pct : 100}%`, background: color }}
      />
    </div>
  );
}

export default function SpeedTestPage() {
  const [results, setResults] = useState<SpeedTestResult[]>([]);
  const [isTesting, setIsTesting] = useState(false);
  const [done, setDone] = useState(false);

  const runTest = async () => {
    setIsTesting(true);
    setDone(false);
    setResults([]);
    try {
      const data = await api.speedtest.run() as SpeedTestResult[];
      // Sort: ok first by latency, then timeout, then error
      data.sort((a, b) => {
        if (a.status === "ok" && b.status === "ok") return (a.latency_ms ?? 9999) - (b.latency_ms ?? 9999);
        if (a.status === "ok") return -1;
        if (b.status === "ok") return 1;
        return 0;
      });
      setResults(data);
      setDone(true);
    } catch (e) {
      console.error(e);
    } finally {
      setIsTesting(false);
    }
  };

  const fastestOk = results.find(r => r.status === "ok" && r.latency_ms !== null);

  return (
    <div className="speedtest-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">端点延迟测速</h1>
          <p className="page-subtitle">
            实测各主流平台 API 端点的响应速度，选择当下最快节点
          </p>
        </div>
        <button
          className={`btn btn-primary ${isTesting ? "btn-loading" : ""}`}
          onClick={runTest}
          disabled={isTesting}
          id="run-speedtest-btn"
        >
          {isTesting ? (
            <><span className="animate-spin">⟳</span> 测速中...</>
          ) : (
            "▶ 开始测速"
          )}
        </button>
      </div>

      <div className="speedtest-body">
        {!done && !isTesting && (
          <div className="empty-state">
            <div className="empty-state-icon">⚡</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>点击「开始测速」</h3>
            <p>将对 9 个主流平台端点并发发送探测请求，通常在 10 秒内完成</p>
          </div>
        )}

        {isTesting && (
          <div className="empty-state">
            <div className="speedtest-pulse">⚡</div>
            <h3 style={{ color: "var(--color-accent)" }}>正在测速...</h3>
            <p>对各端点发送并发探测，请稍候</p>
          </div>
        )}

        {done && results.length > 0 && (
          <>
            {fastestOk && (
              <div className="speedtest-winner card animate-fade-in">
                <div className="winner-label">🏆 当前最快</div>
                <div className="winner-name">{PLATFORM_LABELS[fastestOk.platform] ?? fastestOk.platform}</div>
                <div className="winner-latency" style={{ color: getLatencyColor(fastestOk.latency_ms, fastestOk.status) }}>
                  {fastestOk.latency_ms} ms
                </div>
              </div>
            )}

            <div className="speedtest-list">
              {results.map((r) => (
                <div key={r.platform} className={`speedtest-item card animate-fade-in ${r.status !== "ok" ? "item-error" : ""}`}>
                  <div className="speedtest-item-left">
                    <span className="platform-icon-lg" style={{ color: getLatencyColor(r.latency_ms, r.status) }}>
                      {PLATFORM_ICONS[r.platform] ?? "◌"}
                    </span>
                    <div>
                      <div className="speedtest-platform-name">{PLATFORM_LABELS[r.platform] ?? r.platform}</div>
                      <div className="speedtest-endpoint text-muted">{r.endpoint}</div>
                    </div>
                  </div>
                  <div className="speedtest-item-right">
                    <LatencyBar ms={r.latency_ms} status={r.status} />
                    <div className="speedtest-latency" style={{ color: getLatencyColor(r.latency_ms, r.status) }}>
                      {getLatencyLabel(r.latency_ms, r.status)}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
