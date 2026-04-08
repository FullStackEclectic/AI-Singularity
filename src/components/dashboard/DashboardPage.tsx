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

const CHART_COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

export default function DashboardPage() {
  const { data: metrics, isLoading } = useQuery({
    queryKey: ["dashboard-metrics"],
    queryFn: () => api.analytics.getDashboardMetrics(7),
    refetchInterval: 15_000,
  });

  const STATS_ITEMS = [
    { label: "今日预估开销 (USD)", value: isLoading ? "—" : `$${(metrics?.total_cost_today_usd ?? 0).toFixed(4)}`, icon: "💰", color: "var(--color-primary)" },
    { label: "可用据点 (Active IDEs)", value: isLoading ? "—" : String(metrics?.active_ide_accounts ?? 0), icon: "📡", color: "var(--color-success)" },
    { label: "今日消耗 (Tokens)", value: isLoading ? "—" : String(metrics?.today_total_tokens ?? 0), icon: "⚡", color: "var(--color-warning)" },
    { label: "风控阵亡率 (403/429)", value: isLoading ? "—" : `${((metrics?.forbidden_accounts_ratio ?? 0) * 100).toFixed(1)}%`, icon: "⚠️", color: "var(--color-danger)" },
  ];

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h1 className="page-title glow-text">系统总览与计费测算大盘</h1>
          <p className="page-subtitle">算力矩阵实时运行状态与消耗成本分析</p>
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
          <div className="charts-grid advanced-charts-grid">
            {/* 过去7天的消耗与成本趋势 */}
            <div className="card chart-panel span-2">
              <h3 className="chart-title">资金与 Token 消耗趋势 (7 Days)</h3>
              <div style={{ width: '100%', height: 220 }}>
                <ResponsiveContainer>
                  <AreaChart data={metrics.token_trends} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                    <defs>
                      <linearGradient id="colorCost" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-danger)" stopOpacity={0.4}/>
                        <stop offset="95%" stopColor="var(--color-danger)" stopOpacity={0}/>
                      </linearGradient>
                      <linearGradient id="colorTotal" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="var(--color-accent)" stopOpacity={0.3}/>
                        <stop offset="95%" stopColor="var(--color-accent)" stopOpacity={0}/>
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" vertical={false} />
                    <XAxis dataKey="date" stroke="var(--color-border-active)" tick={{fill: 'var(--color-text-muted)'}} tickLine={false} axisLine={false} />
                    <YAxis yAxisId="left" stroke="var(--color-border-active)" tick={{fill: 'var(--color-text-muted)'}} tickLine={false} axisLine={false} />
                    <YAxis yAxisId="right" orientation="right" stroke="var(--color-danger)" tick={{fill: 'var(--color-danger)'}} tickLine={false} axisLine={false} />
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#1a1b26', borderColor: 'var(--color-border)', borderRadius: '8px', boxShadow: '0 4px 6px -1px rgba(0,0,0, 0.5)' }}
                      itemStyle={{ color: 'var(--color-text-secondary)', fontSize: '13px' }}
                      labelStyle={{ color: 'var(--color-text-primary)', fontWeight: '600', marginBottom: '4px' }}
                    />
                    <Legend iconType="circle" wrapperStyle={{ fontSize: '13px', paddingTop: '10px' }} />
                    <Area yAxisId="left" type="monotone" name="总吞吐 (Tokens)" dataKey="total_tokens" stroke="var(--color-accent)" strokeWidth={2} fillOpacity={1} fill="url(#colorTotal)" />
                    <Area yAxisId="right" type="monotone" name="算力开销 ($USD)" dataKey="total_cost_usd" stroke="var(--color-danger)" strokeWidth={2} fillOpacity={1} fill="url(#colorCost)" />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* 模型重金开销排行榜 */}
            <div className="card chart-panel">
              <h3 className="chart-title">模态成本看板 (Top Model Costs)</h3>
              <div style={{ width: '100%', height: 220 }}>
                <ResponsiveContainer>
                  <BarChart data={metrics.model_costs} layout="vertical" margin={{ top: 10, right: 30, left: 40, bottom: 0 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" horizontal={false} />
                    <XAxis type="number" stroke="var(--color-border-active)" tickLine={false} axisLine={false} />
                    <YAxis dataKey="model_name" type="category" stroke="var(--color-border-active)" tickLine={false} axisLine={false} fontSize={11} width={100} />
                    <Tooltip 
                        cursor={{fill: 'var(--color-bg-hover)'}}
                        formatter={(val: any) => `$${Number(val || 0).toFixed(4)}`}
                        contentStyle={{ backgroundColor: '#1a1b26', borderColor: 'var(--color-border)', borderRadius: '8px' }} 
                        itemStyle={{ color: 'var(--color-primary)' }}
                    />
                    <Bar dataKey="total_cost_usd" name="开销 (USD)" fill="var(--color-primary)" barSize={16} radius={[0, 4, 4, 0]}>
                      {metrics.model_costs.map((entry: any, index: number) => (
                         <Cell key={`cell-${index}`} fill={CHART_COLORS[index % CHART_COLORS.length]} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* 渠道资金流向 (Platform Costs) */}
            <div className="card chart-panel">
              <h3 className="chart-title">渠道资金流向 (Platform Costs)</h3>
              <div style={{ width: '100%', height: 220 }}>
                <ResponsiveContainer>
                  <PieChart>
                    <Pie
                      data={metrics.platform_costs.filter((d: any) => d.total_cost_usd > 0)}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={90}
                      paddingAngle={4}
                      dataKey="total_cost_usd"
                      nameKey="platform"
                      stroke="transparent"
                      cornerRadius={4}
                    >
                      {metrics.platform_costs.map((entry: any, index: number) => (
                        <Cell key={`cell-${index}`} fill={CHART_COLORS[(index + 3) % CHART_COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip 
                      formatter={(val: any) => `$${Number(val || 0).toFixed(4)}`}
                      contentStyle={{ backgroundColor: '#1a1b26', borderColor: 'var(--color-border)', borderRadius: '8px' }} 
                    />
                    <Legend iconType="circle" wrapperStyle={{ fontSize: '13px' }} />
                  </PieChart>
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
                      contentStyle={{ backgroundColor: '#1a1b26', borderColor: 'var(--color-border)', borderRadius: '8px' }} 
                      itemStyle={{ color: 'var(--color-text-secondary)' }}
                    />
                    <Legend iconType="circle" wrapperStyle={{ fontSize: '13px' }} />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </div>

          </div>
        )}
      </div>
    </div>
  );
}
