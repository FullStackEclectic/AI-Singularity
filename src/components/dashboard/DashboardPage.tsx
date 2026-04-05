import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { 
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend,
  BarChart, Bar
} from "recharts";
import "./DashboardPage.css";

const STATUS_COLORS: Record<string, string> = {
  "Active": "var(--color-success)",    
  "Forbidden": "var(--color-danger)", 
  "Expired": "var(--color-warning)",  
  "RateLimited": "var(--color-info)"
};

export default function DashboardPage() {
  const { data: metrics, isLoading } = useQuery({
    queryKey: ["dashboard-metrics"],
    queryFn: () => api.analytics.getDashboardMetrics(7),
    refetchInterval: 15_000,
  });

  const STATS_ITEMS = [
    { label: "可用据点 (Active IDEs)", value: isLoading ? "—" : String(metrics?.active_ide_accounts ?? 0), icon: "📡", color: "var(--color-success)" },
    { label: "终端总数 (Tokens)", value: isLoading ? "—" : String(metrics?.total_user_tokens ?? 0), icon: "🛡️", color: "var(--color-accent)" },
    { label: "今日消耗 (Tokens)", value: isLoading ? "—" : String(metrics?.today_total_tokens ?? 0), icon: "⚡", color: "var(--color-warning)" },
    { label: "风控阵亡率 (403/429)", value: isLoading ? "—" : `${((metrics?.forbidden_accounts_ratio ?? 0) * 100).toFixed(1)}%`, icon: "⚠️", color: "var(--color-danger)" },
  ];

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h1 className="page-title">系统总览</h1>
          <p className="page-subtitle">算力矩阵实时运行状态与消耗分析</p>
        </div>
      </div>

      <div className="dashboard-body">
        {/* 指标卡片 */}
        <div className="stats-grid">
          {STATS_ITEMS.map((s) => (
            <div key={s.label} className={`card stat-card ${isLoading ? "loading" : ""}`}>
              <div className="stat-icon" style={{ color: s.color }}>{s.icon}</div>
              <div className="stat-value">{s.value}</div>
              <div className="stat-label">{s.label}</div>
            </div>
          ))}
        </div>

        {metrics && (
          <div className="charts-grid">
            {/* 过去7天的吞吐趋势 */}
            <div className="card chart-panel span-2">
              <h3 className="chart-title">消耗趋势 (7 Days)</h3>
              <div style={{ width: '100%', height: 180 }}>
                <ResponsiveContainer>
                  <AreaChart data={metrics.token_trends} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                    <defs>
                      <linearGradient id="colorTotal" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-accent)" stopOpacity={0.3}/>
                        <stop offset="95%" stopColor="var(--color-accent)" stopOpacity={0}/>
                      </linearGradient>
                      <linearGradient id="colorPrompt" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-info)" stopOpacity={0.3}/>
                        <stop offset="95%" stopColor="var(--color-info)" stopOpacity={0}/>
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" vertical={false} />
                    <XAxis dataKey="date" stroke="var(--color-border-active)" tick={{fill: 'var(--color-text-muted)'}} tickLine={false} axisLine={false} />
                    <YAxis stroke="var(--color-border-active)" tick={{fill: 'var(--color-text-muted)'}} tickLine={false} axisLine={false} />
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#ffffff', borderColor: 'var(--color-border)', borderRadius: '8px', boxShadow: '0 4px 6px -1px rgba(15, 23, 42, 0.05)' }}
                      itemStyle={{ color: 'var(--color-text-secondary)', fontSize: '13px' }}
                      labelStyle={{ color: 'var(--color-text-primary)', fontWeight: '600', marginBottom: '4px' }}
                    />
                    <Legend iconType="circle" wrapperStyle={{ fontSize: '13px', paddingTop: '10px' }} />
                    <Area type="monotone" name="提示词 (Prompt)" dataKey="prompt_tokens" stroke="var(--color-info)" strokeWidth={2} fillOpacity={1} fill="url(#colorPrompt)" />
                    <Area type="monotone" name="总吞吐 (Total)" dataKey="total_tokens" stroke="var(--color-accent)" strokeWidth={2} fillOpacity={1} fill="url(#colorTotal)" />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* IDE 账号健康度分布 */}
            <div className="card chart-panel">
              <h3 className="chart-title">设备池健康度 (IDE Fleet Status)</h3>
              <div style={{ width: '100%', height: 220 }}>
                <ResponsiveContainer>
                  <PieChart>
                    <Pie
                      data={metrics.ide_status_distribution.filter((d: any) => d.value > 0)}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={90}
                      paddingAngle={4}
                      dataKey="value"
                      stroke="transparent"
                      cornerRadius={4}
                    >
                      {
                        metrics.ide_status_distribution.map((entry: any, index: number) => (
                          <Cell key={`cell-${index}`} fill={STATUS_COLORS[entry.name] || "var(--color-text-muted)"} />
                        ))
                      }
                    </Pie>
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#ffffff', borderColor: 'var(--color-border)', borderRadius: '8px', boxShadow: '0 4px 6px -1px rgba(15, 23, 42, 0.05)' }} 
                      itemStyle={{ color: 'var(--color-text-secondary)' }}
                    />
                    <Legend iconType="circle" wrapperStyle={{ fontSize: '13px' }} />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* 顶部消耗排行榜 */}
            <div className="card chart-panel">
              <h3 className="chart-title">终端消耗追踪 (Top Consumers)</h3>
              <div style={{ width: '100%', height: 220 }}>
                <ResponsiveContainer>
                  <BarChart data={metrics.top_consumers} layout="vertical" margin={{ top: 10, right: 30, left: 20, bottom: 0 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" horizontal={false} />
                    <XAxis type="number" stroke="var(--color-border-active)" tickLine={false} axisLine={false} />
                    <YAxis dataKey="client_app" type="category" stroke="var(--color-border-active)" tickLine={false} axisLine={false} fontSize={12} width={80} />
                    <Tooltip 
                        cursor={{fill: 'var(--color-bg-hover)'}}
                        contentStyle={{ backgroundColor: '#ffffff', borderColor: 'var(--color-border)', borderRadius: '8px', boxShadow: '0 4px 6px -1px rgba(15, 23, 42, 0.05)' }} 
                        itemStyle={{ color: 'var(--color-text-secondary)' }}
                    />
                    <Bar dataKey="total_tokens" name="消耗量" fill="var(--color-accent)" barSize={16} radius={[0, 4, 4, 0]} />
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
