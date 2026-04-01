import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import "./DashboardPage.css";

const PLATFORMS = [
  { name: "OpenAI",    color: "var(--color-openai)",    icon: "◎", key: "open_ai"  },
  { name: "Anthropic", color: "var(--color-anthropic)", icon: "◈", key: "anthropic" },
  { name: "Gemini",    color: "var(--color-gemini)",    icon: "◆", key: "gemini"    },
  { name: "DeepSeek",  color: "var(--color-deepseek)",  icon: "◉", key: "deep_seek" },
  { name: "百炼",      color: "#f27c3f",                icon: "◐", key: "aliyun"    },
  { name: "豆包",      color: "#30b0c7",                icon: "◑", key: "bytedance" },
  { name: "Kimi",      color: "#6366f1",                icon: "◒", key: "moonshot"  },
  { name: "智谱 GLM",  color: "#8b5cf6",                icon: "◓", key: "zhipu"    },
];

const ALERT_ICONS: Record<string, string> = {
  critical: "🔴",
  warning:  "🟡",
  info:     "🔵",
};

const ALERT_CLASS: Record<string, string> = {
  critical: "alert-item-critical",
  warning:  "alert-item-warning",
  info:     "alert-item-info",
};

export default function DashboardPage() {
  const { data: stats, isLoading } = useQuery({
    queryKey: ["dashboard-stats"],
    queryFn: api.stats.getDashboard,
    refetchInterval: 30_000,
  });

  const { data: keys = [] } = useQuery({
    queryKey: ["keys"],
    queryFn: api.keys.list,
  });

  const { data: alerts = [], isLoading: alertsLoading } = useQuery({
    queryKey: ["alerts"],
    queryFn: api.alerts.get,
    refetchInterval: 60_000,
  });

  // 按平台统计 Key 数量
  const keysByPlatform = keys.reduce<Record<string, number>>((acc, k: any) => {
    acc[k.platform] = (acc[k.platform] || 0) + 1;
    return acc;
  }, {});

  const criticalCount = alerts.filter((a: any) => a.level === "critical").length;
  const warningCount  = alerts.filter((a: any) => a.level === "warning").length;

  const STATS_ITEMS = [
    { label: "已配置账号", value: isLoading ? "—" : String(stats?.total_keys ?? 0),                       icon: "🔑", color: "var(--color-accent)"   },
    { label: "有效 Key",   value: isLoading ? "—" : String(stats?.valid_keys ?? 0),                        icon: "✅", color: "var(--color-success)"  },
    { label: "本月消耗",   value: isLoading ? "—" : `$${(stats?.total_cost_usd ?? 0).toFixed(2)}`,         icon: "💰", color: "var(--color-warning)"  },
    { label: "活跃告警",   value: alertsLoading ? "—" : String(criticalCount + warningCount),              icon: criticalCount > 0 ? "🚨" : "🔔", color: criticalCount > 0 ? "var(--color-danger)" : "var(--color-info)" },
  ];

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h1 className="page-title">总览</h1>
          <p className="page-subtitle">所有 AI 资源的一站式控制中心</p>
        </div>
        <div className="dashboard-status-bar">
          <span className={`status-dot ${criticalCount > 0 ? "invalid" : "valid"}`} />
          <span className="text-secondary" style={{ fontSize: 13 }}>
            {criticalCount > 0 ? `${criticalCount} 项紧急告警` : "系统正常"}
          </span>
        </div>
      </div>

      <div className="dashboard-body">
        {/* 统计卡片 */}
        <div className="stats-grid">
          {STATS_ITEMS.map((s) => (
            <div key={s.label} className={`stat-card card animate-fade-in ${isLoading ? "loading" : ""}`}>
              <div className="stat-icon" style={{ color: s.color }}>{s.icon}</div>
              <div className="stat-value">{s.value}</div>
              <div className="stat-label text-muted">{s.label}</div>
            </div>
          ))}
        </div>

        {/* 告警面板 */}
        {!alertsLoading && alerts.length > 0 && (
          <div className="alerts-section">
            <h2 className="section-title" style={{ marginTop: "var(--space-6)" }}>
              🔔 当前告警
            </h2>
            <div className="alerts-list">
              {(alerts as any[]).map((a) => (
                <div key={a.id} className={`alert-item card ${ALERT_CLASS[a.level] ?? ""} animate-fade-in`}>
                  <div className="alert-icon">{ALERT_ICONS[a.level] ?? "ℹ️"}</div>
                  <div className="alert-content">
                    <div className="alert-title">{a.title}</div>
                    <div className="alert-message text-muted">{a.message}</div>
                  </div>
                  {a.platform && (
                    <div className="badge badge-muted" style={{ flexShrink: 0 }}>{a.platform}</div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 平台支持矩阵 */}
        <div className="card" style={{ marginTop: "var(--space-6)" }}>
          <h2 className="section-title">平台一览</h2>
          <div className="platform-grid">
            {PLATFORMS.map((p) => {
              const count = keysByPlatform[p.key] ?? 0;
              return (
                <div key={p.name} className="platform-item">
                  <span className="platform-icon" style={{ color: p.color }}>{p.icon}</span>
                  <span className="platform-name">{p.name}</span>
                  {count > 0 ? (
                    <span className="badge badge-success" style={{ marginLeft: "auto" }}>
                      {count} 个 Key
                    </span>
                  ) : (
                    <span className="badge badge-muted" style={{ marginLeft: "auto" }}>
                      未配置
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* 快速开始（仅无 Key 时展示） */}
        {(stats?.total_keys ?? 0) === 0 && !isLoading && (
          <div className="card card-accent" style={{ marginTop: "var(--space-6)" }}>
            <div className="quick-start">
              <div className="quick-start-icon">✦</div>
              <div className="quick-start-content">
                <h3>开始使用 AI Singularity</h3>
                <p className="text-secondary">
                  添加你的第一个 API Key，开始统一管理所有 AI 资源
                </p>
              </div>
              <a href="#keys" className="btn btn-primary">＋ 添加 API Key</a>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
