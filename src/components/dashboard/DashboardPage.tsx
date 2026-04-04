import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { 
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend,
  BarChart, Bar
} from "recharts";
import "./DashboardPage.css";

const STATUS_COLORS: Record<string, string> = {
  "Active": "#10b981",    // Aurora Green
  "Forbidden": "#ff4d4f", // Alert Red
  "Expired": "#f59e0b",   // Warning Amber
  "RateLimited": "#6366f1"
};

export default function DashboardPage() {
  const { data: metrics, isLoading } = useQuery({
    queryKey: ["dashboard-metrics"],
    queryFn: () => api.analytics.getDashboardMetrics(7),
    refetchInterval: 15_000,
  });

  const STATS_ITEMS = [
    { label: "可用指纹据点", value: isLoading ? "—" : String(metrics?.active_ide_accounts ?? 0), icon: "☢️", color: "var(--color-primary)" },
    { label: "下发终端总数", value: isLoading ? "—" : String(metrics?.total_user_tokens ?? 0), icon: "🛡️", color: "var(--color-info)" },
    { label: "今日吞吐量 (Tokens)", value: isLoading ? "—" : String(metrics?.today_total_tokens ?? 0), icon: "🔥", color: "var(--color-warning)" },
    { label: "阵亡率 (403/429)", value: isLoading ? "—" : `${((metrics?.forbidden_accounts_ratio ?? 0) * 100).toFixed(1)}%`, icon: "⚠️", color: "var(--color-danger)" },
  ];

  return (
    <div className="dashboard cyberpunk-theme">
      <div className="page-header glow-border-bottom">
        <div>
          <h1 className="page-title cyber-glitch" data-text="总览雷达">总览雷达</h1>
          <p className="page-subtitle text-muted">全域算力引擎实时监控矩阵</p>
        </div>
      </div>

      <div className="dashboard-body">
        {/* 指标卡片 */}
        <div className="stats-grid">
          {STATS_ITEMS.map((s) => (
            <div key={s.label} className={`stat-card hex-border glassmorphism animate-pulse-glow ${isLoading ? "loading" : ""}`}>
              <div className="stat-icon" style={{ textShadow: `0 0 10px ${s.color}`, color: s.color }}>{s.icon}</div>
              <div className="stat-value neon-text">{s.value}</div>
              <div className="stat-label text-muted">{s.label}</div>
            </div>
          ))}
        </div>

        {metrics && (
          <div className="charts-grid">
            {/* 过去7天的吞吐趋势 */}
            <div className="chart-panel span-2 glassmorphism border-cyan">
              <h3 className="chart-title">▶ 算力潮汐折线 / 7-DAY THROUGHPUT</h3>
              <div style={{ width: '100%', height: 300 }}>
                <ResponsiveContainer>
                  <AreaChart data={metrics.token_trends} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                    <defs>
                      <linearGradient id="colorTotal" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-cyan)" stopOpacity={0.8}/>
                        <stop offset="95%" stopColor="var(--color-cyan)" stopOpacity={0}/>
                      </linearGradient>
                      <linearGradient id="colorPrompt" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-purple)" stopOpacity={0.8}/>
                        <stop offset="95%" stopColor="var(--color-purple)" stopOpacity={0}/>
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="#334155" opacity={0.4} />
                    <XAxis dataKey="date" stroke="#94a3b8" />
                    <YAxis stroke="#94a3b8" />
                    <Tooltip 
                      contentStyle={{ backgroundColor: 'rgba(15, 23, 42, 0.9)', borderColor: 'var(--color-cyan)', backdropFilter: 'blur(4px)' }}
                      itemStyle={{ color: '#e2e8f0' }}
                    />
                    <Legend />
                    <Area type="monotone" name="提示词 (Prompt)" dataKey="prompt_tokens" stroke="var(--color-purple)" fillOpacity={1} fill="url(#colorPrompt)" />
                    <Area type="monotone" name="总吞吐 (Total)" dataKey="total_tokens" stroke="var(--color-cyan)" fillOpacity={1} fill="url(#colorTotal)" />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* IDE 账号健康度分布 */}
            <div className="chart-panel glassmorphism border-emerald">
              <h3 className="chart-title">▶ 兵工厂态势 / IDE FLEET STATUS</h3>
              <div style={{ width: '100%', height: 300 }}>
                <ResponsiveContainer>
                  <PieChart>
                    <Pie
                      data={metrics.ide_status_distribution.filter((d: any) => d.value > 0)}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={90}
                      paddingAngle={5}
                      dataKey="value"
                      stroke="none"
                    >
                      {
                        metrics.ide_status_distribution.map((entry: any, index: number) => (
                          <Cell key={`cell-${index}`} fill={STATUS_COLORS[entry.name] || "#64748b"} />
                        ))
                      }
                    </Pie>
                    <Tooltip 
                      contentStyle={{ backgroundColor: 'rgba(15, 23, 42, 0.9)', borderColor: 'var(--color-emerald)' }} 
                    />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* 顶部消耗排行榜 */}
            <div className="chart-panel glassmorphism border-orange">
              <h3 className="chart-title">▶ 终端消耗排名 / TOP CONSUMERS</h3>
              <div style={{ width: '100%', height: 300 }}>
                <ResponsiveContainer>
                  <BarChart data={metrics.top_consumers} layout="vertical" margin={{ top: 10, right: 30, left: 40, bottom: 0 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#334155" opacity={0.4} horizontal={false} />
                    <XAxis type="number" stroke="#94a3b8" />
                    <YAxis dataKey="client_app" type="category" stroke="#94a3b8" fontSize={12} width={80} />
                    <Tooltip 
                        cursor={{fill: 'transparent'}}
                        contentStyle={{ backgroundColor: 'rgba(15, 23, 42, 0.9)', borderColor: 'var(--color-warning)' }} 
                    />
                    <Bar dataKey="total_tokens" name="消耗量" fill="var(--color-warning)" barSize={20} radius={[0, 4, 4, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
