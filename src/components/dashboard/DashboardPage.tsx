import "./DashboardPage.css";

const STATS = [
  { label: "已配置账号", value: "0", icon: "🔑", color: "var(--color-accent)" },
  { label: "有效 Key", value: "0", icon: "✅", color: "var(--color-success)" },
  { label: "本月消耗", value: "$0.00", icon: "💰", color: "var(--color-warning)" },
  { label: "支持平台", value: "11", icon: "🌐", color: "var(--color-info)" },
];

const PLATFORMS = [
  { name: "OpenAI", color: "var(--color-openai)", icon: "◎" },
  { name: "Anthropic", color: "var(--color-anthropic)", icon: "◈" },
  { name: "Gemini", color: "var(--color-gemini)", icon: "◆" },
  { name: "DeepSeek", color: "var(--color-deepseek)", icon: "◉" },
  { name: "百炼", color: "#f27c3f", icon: "◐" },
  { name: "豆包", color: "#30b0c7", icon: "◑" },
  { name: "Kimi", color: "#6366f1", icon: "◒" },
  { name: "智谱", color: "#8b5cf6", icon: "◓" },
];

export default function DashboardPage() {
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
          {STATS.map((s) => (
            <div key={s.label} className="stat-card card">
              <div className="stat-icon" style={{ color: s.color }}>{s.icon}</div>
              <div className="stat-value">{s.value}</div>
              <div className="stat-label text-muted">{s.label}</div>
            </div>
          ))}
        </div>

        {/* 平台支持矩阵 */}
        <div className="card" style={{ marginTop: "var(--space-6)" }}>
          <h2 className="section-title">支持的平台</h2>
          <div className="platform-grid">
            {PLATFORMS.map((p) => (
              <div key={p.name} className="platform-item">
                <span className="platform-icon" style={{ color: p.color }}>{p.icon}</span>
                <span className="platform-name">{p.name}</span>
                <span className="badge badge-muted" style={{ marginLeft: "auto" }}>未配置</span>
              </div>
            ))}
          </div>
        </div>

        {/* 快速开始 */}
        <div className="card card-accent" style={{ marginTop: "var(--space-6)" }}>
          <div className="quick-start">
            <div className="quick-start-icon">✦</div>
            <div className="quick-start-content">
              <h3>开始使用 AI Singularity</h3>
              <p className="text-secondary">添加你的第一个 API Key，开始统一管理所有 AI 资源</p>
            </div>
            <a href="#keys" className="btn btn-primary">
              ＋ 添加 API Key
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
