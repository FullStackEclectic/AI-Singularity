import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import type { AnnouncementAction, CurrentAccountSnapshot } from "../../lib/api";
import { formatIdePlatformKeyLabel } from "../accounts/unifiedAccountsUtils";
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend,
  BarChart, Bar,
} from "recharts";
import { useTranslation } from "react-i18next";
import "./DashboardPage.css";

const STATUS_COLORS: Record<string, string> = {
  Active:      "var(--color-success)",
  Forbidden:   "var(--color-danger)",
  Expired:     "var(--color-warning)",
  RateLimited: "var(--color-info)",
};

const CHART_COLORS = [
  "#3b82f6", "#10b981", "#f59e0b", "#ef4444",
  "#8b5cf6", "#ec4899", "#14b8a6", "#f97316",
];

// Tooltip 样式（浅色主题）
const TOOLTIP_STYLE = {
  backgroundColor: "var(--color-bg-card)",
  border: "1px solid var(--color-border)",
  borderRadius: "8px",
  boxShadow: "var(--shadow-md)",
  fontSize: "12px",
};
const TOOLTIP_ITEM_STYLE = { color: "var(--color-text-secondary)", fontSize: "12px" };
const TOOLTIP_LABEL_STYLE = { color: "var(--color-text-primary)", fontWeight: 600, marginBottom: 4 };

export default function DashboardPage() {
  const { i18n } = useTranslation();
  const queryClient = useQueryClient();

  const { data: metrics, isLoading } = useQuery({
    queryKey: ["dashboard-metrics"],
    queryFn: () => api.analytics.getDashboardMetrics(7),
    refetchInterval: 15_000,
  });

  const { data: announcementState, isLoading: announcementsLoading } = useQuery({
    queryKey: ["announcement-state", i18n.language],
    queryFn: () => api.announcements.getState(i18n.language),
    staleTime: 60_000,
  });

  const { data: currentSnapshots = [] } = useQuery<CurrentAccountSnapshot[]>({
    queryKey: ["providerCurrentSnapshots"],
    queryFn: () => api.providerCurrent.listSnapshots(),
    refetchInterval: 15_000,
  });

  const STATS_ITEMS = [
    {
      label: "今日预估费用（USD）",
      value: isLoading ? "—" : `$${(metrics?.total_cost_today_usd ?? 0).toFixed(4)}`,
      icon: "💰",
      bg: "rgba(37,99,235,0.08)",
    },
    {
      label: "可用账号数",
      value: isLoading ? "—" : String(metrics?.active_ide_accounts ?? 0),
      icon: "✅",
      bg: "rgba(16,185,129,0.08)",
    },
    {
      label: "今日消耗（Tokens）",
      value: isLoading ? "—" : String(metrics?.today_total_tokens ?? 0),
      icon: "⚡",
      bg: "rgba(245,158,11,0.08)",
    },
    {
      label: "账号异常率",
      value: isLoading ? "—" : `${((metrics?.forbidden_accounts_ratio ?? 0) * 100).toFixed(1)}%`,
      icon: "⚠️",
      bg: "rgba(239,68,68,0.08)",
    },
  ];

  const executeAnnouncementAction = (action?: AnnouncementAction | null) => {
    if (!action?.target) return;
    const actionType = action.type?.trim().toLowerCase();
    const target = action.target.trim();
    if (actionType === "url" || /^https?:\/\//i.test(target)) {
      window.open(target, "_blank", "noopener,noreferrer");
    } else {
      window.dispatchEvent(new CustomEvent("ais:navigate", { detail: target }));
    }
  };

  const hasAnnouncements = (announcementState?.announcements.length ?? 0) > 0;

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h1 className="page-title">系统总览</h1>
          <p className="page-subtitle">实时运行状态与费用分析</p>
        </div>
      </div>

      <div className="dashboard-body">
        {/* ── 左栏：指标 + 账号状态 + 公告 ── */}
        <div className="dashboard-left">
          {/* 4 个指标卡 */}
          <div className="stats-grid">
            {STATS_ITEMS.map((s) => (
              <div key={s.label} className={`stat-card ${isLoading ? "loading" : ""}`}>
                <div className="stat-icon-wrap" style={{ background: s.bg }}>
                  {s.icon}
                </div>
                <div className="stat-text">
                  <div className="stat-value">{s.value}</div>
                  <div className="stat-label">{s.label}</div>
                </div>
              </div>
            ))}
          </div>

          {/* 当前账号状态 */}
          {currentSnapshots.length > 0 && (
            <div className="current-account-panel">
              <div className="current-account-panel-header">
                <span className="chart-title">当前账号状态</span>
                <span className="announcement-panel-meta">{currentSnapshots.length} 个平台</span>
              </div>
              <div className="current-account-list">
                {currentSnapshots.map((item) => (
                  <div key={item.platform} className="current-account-item">
                    <div className="current-account-platform">
                      {formatIdePlatformKeyLabel(item.platform)}
                    </div>
                    <div className="current-account-main">
                      <div className="current-account-label">
                        {item.label || "未设置当前账号"}
                      </div>
                      <div className="current-account-meta">
                        <span>{item.email || "—"}</span>
                        <span style={{
                          color: item.status === "active" ? "var(--color-success)"
                            : item.status === "forbidden" ? "var(--color-danger)"
                            : "var(--color-text-muted)"
                        }}>
                          {item.status || "unknown"}
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 公告 */}
          {(hasAnnouncements || announcementsLoading) && (
            <div className="announcement-panel">
              <div className="announcement-panel-header">
                <span className="chart-title">系统公告</span>
                <div className="announcement-panel-actions">
                  {(announcementState?.unreadIds.length ?? 0) > 0 && (
                    <button
                      className="btn btn-ghost btn-sm"
                      onClick={async () => {
                        await api.announcements.markAllRead(i18n.language);
                        queryClient.invalidateQueries({ queryKey: ["announcement-state", i18n.language] });
                      }}
                    >
                      全部已读
                    </button>
                  )}
                </div>
              </div>

              {announcementsLoading ? (
                <div className="announcement-panel-meta">加载中...</div>
              ) : (
                <div className="announcement-list">
                  {announcementState!.announcements.map((item) => {
                    const unread = announcementState!.unreadIds.includes(item.id);
                    return (
                      <div key={item.id} className={`announcement-item ${unread ? "unread" : ""}`}>
                        <div className="announcement-item-top">
                          <div className="announcement-item-title-row">
                            <span className={`announcement-type ${item.type || "info"}`}>
                              {item.type || "info"}
                            </span>
                            <span className="announcement-item-title">{item.title}</span>
                          </div>
                          <span className="announcement-item-time">
                            {item.createdAt ? new Date(item.createdAt).toLocaleDateString() : ""}
                          </span>
                        </div>
                        {item.summary && (
                          <div className="announcement-item-summary">{item.summary}</div>
                        )}
                        <div className="announcement-item-actions">
                          {item.action?.target && (
                            <button
                              className="btn btn-ghost btn-sm"
                              onClick={() => executeAnnouncementAction(item.action)}
                            >
                              {item.action.label || "查看详情"}
                            </button>
                          )}
                          {unread && (
                            <button
                              className="btn btn-ghost btn-sm"
                              onClick={async () => {
                                await api.announcements.markRead(item.id);
                                queryClient.invalidateQueries({ queryKey: ["announcement-state", i18n.language] });
                              }}
                            >
                              标为已读
                            </button>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>

        {/* ── 右栏：4 个图表 2×2 ── */}
        <div className="dashboard-right">
          {!metrics && !isLoading && (
            <div className="dashboard-empty">暂无数据，稍后自动刷新</div>
          )}
          {isLoading && !metrics && (
            <div className="dashboard-empty animate-pulse">加载中...</div>
          )}
          {metrics && (
            <div className="charts-grid">
              {/* Token 消耗趋势 — 占满上方两列 */}
              <div className="chart-panel chart-panel-full">
                <h3 className="chart-title">Token 消耗与费用趋势（近 7 天）</h3>
                <div className="chart-body">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                      data={metrics.token_trends}
                      margin={{ top: 6, right: 24, left: 0, bottom: 0 }}
                    >
                      <defs>
                        <linearGradient id="gCost" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%"  stopColor="#ef4444" stopOpacity={0.25} />
                          <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                        </linearGradient>
                        <linearGradient id="gToken" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%"  stopColor="#3b82f6" stopOpacity={0.2} />
                          <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                        </linearGradient>
                      </defs>
                      <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" vertical={false} />
                      <XAxis dataKey="date" tick={{ fill: "var(--color-text-muted)", fontSize: 11 }} tickLine={false} axisLine={false} />
                      <YAxis yAxisId="left"  tick={{ fill: "var(--color-text-muted)", fontSize: 11 }} tickLine={false} axisLine={false} />
                      <YAxis yAxisId="right" orientation="right" tick={{ fill: "#ef4444", fontSize: 11 }} tickLine={false} axisLine={false} />
                      <Tooltip contentStyle={TOOLTIP_STYLE} itemStyle={TOOLTIP_ITEM_STYLE} labelStyle={TOOLTIP_LABEL_STYLE} />
                      <Legend iconType="circle" wrapperStyle={{ fontSize: 12, paddingTop: 6 }} />
                      <Area yAxisId="left"  type="monotone" name="Token 消耗" dataKey="total_tokens"  stroke="#3b82f6" strokeWidth={2} fill="url(#gToken)" />
                      <Area yAxisId="right" type="monotone" name="费用（USD）" dataKey="total_cost_usd" stroke="#ef4444" strokeWidth={2} fill="url(#gCost)" />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* 模型费用排行 */}
              <div className="chart-panel">
                <h3 className="chart-title">模型费用排行</h3>
                <div className="chart-body">
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                      data={metrics.model_costs}
                      layout="vertical"
                      margin={{ top: 4, right: 20, left: 0, bottom: 0 }}
                    >
                      <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" horizontal={false} />
                      <XAxis type="number" tick={{ fill: "var(--color-text-muted)", fontSize: 11 }} tickLine={false} axisLine={false} tickFormatter={(v) => `$${v}`} />
                      <YAxis dataKey="model_name" type="category" tick={{ fill: "var(--color-text-muted)", fontSize: 11 }} tickLine={false} axisLine={false} width={90} />
                      <Tooltip
                        cursor={{ fill: "var(--color-bg-hover)" }}
                        formatter={(val: any) => [`$${Number(val || 0).toFixed(4)}`, "费用"]}
                        contentStyle={TOOLTIP_STYLE}
                        itemStyle={TOOLTIP_ITEM_STYLE}
                        labelStyle={TOOLTIP_LABEL_STYLE}
                      />
                      <Bar dataKey="total_cost_usd" name="费用（USD）" barSize={14} radius={[0, 4, 4, 0]}>
                        {metrics.model_costs.map((_: any, i: number) => (
                          <Cell key={i} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                        ))}
                      </Bar>
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* 各平台费用占比 */}
              <div className="chart-panel">
                <h3 className="chart-title">各平台费用占比</h3>
                <div className="chart-body">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <Pie
                        data={metrics.platform_costs.filter((d: any) => d.total_cost_usd > 0)}
                        cx="50%" cy="45%"
                        innerRadius="38%" outerRadius="60%"
                        paddingAngle={3}
                        dataKey="total_cost_usd"
                        nameKey="platform"
                        stroke="transparent"
                        cornerRadius={3}
                      >
                        {metrics.platform_costs.map((_: any, i: number) => (
                          <Cell key={i} fill={CHART_COLORS[(i + 2) % CHART_COLORS.length]} />
                        ))}
                      </Pie>
                      <Tooltip
                        formatter={(val: any) => [`$${Number(val || 0).toFixed(4)}`, "费用"]}
                        contentStyle={TOOLTIP_STYLE}
                        itemStyle={TOOLTIP_ITEM_STYLE}
                      />
                      <Legend iconType="circle" wrapperStyle={{ fontSize: 11 }} />
                    </PieChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* 账号状态分布 */}
              <div className="chart-panel">
                <h3 className="chart-title">账号状态分布</h3>
                <div className="chart-body">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <Pie
                        data={metrics.ide_status_distribution.filter((d: any) => d.value > 0)}
                        cx="50%" cy="45%"
                        innerRadius="38%" outerRadius="60%"
                        paddingAngle={3}
                        dataKey="value"
                        stroke="transparent"
                        cornerRadius={3}
                      >
                        {metrics.ide_status_distribution.map((entry: any, i: number) => (
                          <Cell key={i} fill={STATUS_COLORS[entry.name] || "var(--color-text-muted)"} />
                        ))}
                      </Pie>
                      <Tooltip
                        contentStyle={TOOLTIP_STYLE}
                        itemStyle={TOOLTIP_ITEM_STYLE}
                      />
                      <Legend iconType="circle" wrapperStyle={{ fontSize: 11 }} />
                    </PieChart>
                  </ResponsiveContainer>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
