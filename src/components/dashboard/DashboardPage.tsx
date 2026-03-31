import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import "./DashboardPage.css";

const PLATFORMS = [
  { name: "OpenAI", color: "var(--color-openai)", icon: "◎", key: "open_ai" },
  { name: "Anthropic", color: "var(--color-anthropic)", icon: "◈", key: "anthropic" },
  { name: "Gemini", color: "var(--color-gemini)", icon: "◆", key: "gemini" },
  { name: "DeepSeek", color: "var(--color-deepseek)", icon: "◉", key: "deep_seek" },
  { name: "百炼", color: "#f27c3f", icon: "◐", key: "aliyun" },
  { name: "豆包", color: "#30b0c7", icon: "◑", key: "bytedance" },
  { name: "Kimi", color: "#6366f1", icon: "◒", key: "moonshot" },
  { name: "智谱 GLM", color: "#8b5cf6", icon: "◓", key: "zhipu" },
];

export default function DashboardPage() {
  const { data: stats, isLoading } = useQuery({
    queryKey: ["dashboard-stats"],
    queryFn: api.stats.getDashboard,
    refetchInterval: 30_000, // 每30秒自动刷新
  });

  const { data: keys = [] } = useQuery({
    queryKey: ["keys"],
    queryFn: api.keys.list,
  });

  // 按平台统计 Key 数量
  const keysByPlatform = keys.reduce<Record<string, number>>((acc, k) => {
    acc[k.platform] = (acc[k.platform] || 0) + 1;
    return acc;
  }, {});

  const STATS_ITEMS = [
    {
      label: "已配置账号",
      value: isLoading ? "—" : String(stats?.total_keys ?? 0),
      icon: "🔑",
      color: "var(--color-accent)",
    },
    {
      label: "有效 Key",
      value: isLoading ? "—" : String(stats?.valid_keys ?? 0),
      icon: "✅",
      color: "var(--color-success)",
    },
    {
      label: "本月消耗",
      value: isLoading ? "—" : `$${(stats?.total_cost_usd ?? 0).toFixed(2)}`,
      icon: "💰",
      color: "var(--color-warning)",
    },
    {
      label: "已接入平台",
      value: isLoading ? "—" : String(stats?.total_platforms ?? 0),
      icon: "🌐",
      color: "var(--color-info)",
    },
  ];

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h1 className="page-title">总览</h1>
          <p className="page-subtitle">所有 AI 资源的一站式控制中心</p>
        </div>
        <div className="dashboard-status-bar">
          <span className="status-dot valid" />
          <span className="text-secondary" style={{ fontSize: 13 }}>系统正常</span>
        </div>
      </div>

      <div className="dashboard-body">
        {/* 统计卡片 */}
        <div className="stats-grid">
          {STATS_ITEMS.map((s) => (
            <div key={s.label} className={`stat-card card ${isLoading ? "loading" : ""}`}>
              <div className="stat-icon" style={{ color: s.color }}>{s.icon}</div>
              <div className="stat-value">{s.value}</div>
              <div className="stat-label text-muted">{s.label}</div>
            </div>
          ))}
        </div>

        {/* 状态警示（有无效 Key 时展示） */}
        {(stats?.invalid_keys ?? 0) > 0 && (
          <div className="alert-bar" style={{ marginTop: "var(--space-4)" }}>
            <span>⚠️</span>
            <span>有 <strong>{stats!.invalid_keys}</strong> 个 Key 状态异常，请前往「API Keys」页面检查</span>
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
