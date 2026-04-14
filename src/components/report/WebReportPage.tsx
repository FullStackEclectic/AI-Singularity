import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Download, FileText, RefreshCw } from "lucide-react";
import { api } from "../../lib/api";
import "./WebReportPage.css";

function formatDateTime(value?: string | null) {
  if (!value) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function buildReportHtml(payload: {
  generatedAt: string;
  metrics: any;
  providers: any[];
  ideAccounts: any[];
  balances: any[];
  alerts: any[];
  announcements: any[];
  logs: any[];
}) {
  const { generatedAt, metrics, providers, ideAccounts, balances, alerts, announcements, logs } = payload;
  const escapeHtml = (value: unknown) =>
    String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");

  const providerRows = providers
    .map((item) => `<tr><td>${escapeHtml(item.name)}</td><td>${escapeHtml(item.platform)}</td><td>${item.is_active ? "激活" : "未激活"}</td></tr>`)
    .join("");
  const accountRows = ideAccounts
    .slice(0, 20)
    .map((item) => `<tr><td>${escapeHtml(item.label || item.email)}</td><td>${escapeHtml(item.origin_platform)}</td><td>${escapeHtml(item.status)}</td><td>${escapeHtml(item.project_id || "—")}</td></tr>`)
    .join("");
  const announcementRows = announcements
    .slice(0, 10)
    .map((item) => `<li><strong>${escapeHtml(item.title)}</strong><div>${escapeHtml(item.summary || item.content || "")}</div></li>`)
    .join("");
  const alertRows = alerts
    .slice(0, 10)
    .map((item) => `<li><strong>${escapeHtml(item.title || item.level || "告警")}</strong><div>${escapeHtml(item.message || item.description || "")}</div></li>`)
    .join("");
  const logRows = logs
    .slice(0, 10)
    .map((item) => `<tr><td>${escapeHtml(item.name)}</td><td>${escapeHtml(item.kind)}</td><td>${escapeHtml(item.modified_at || "—")}</td><td>${escapeHtml(item.size)}</td></tr>`)
    .join("");
  const balanceRows = balances
    .slice(0, 12)
    .map((item) => `<tr><td>${escapeHtml(item.provider_name)}</td><td>${escapeHtml(item.platform)}</td><td>${escapeHtml(item.latest_balance_usd ?? "—")}</td><td>${escapeHtml(item.quota_remaining ?? "—")}</td></tr>`)
    .join("");

  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>AI Singularity Web Report</title>
  <style>
    body { font-family: "Segoe UI", Arial, sans-serif; background:#0b1220; color:#e5eefc; margin:0; padding:32px; }
    .wrap { max-width: 1180px; margin: 0 auto; }
    .hero { padding:24px; border-radius:20px; background:linear-gradient(135deg,#13203a,#0f1728); border:1px solid rgba(255,255,255,0.08); }
    h1,h2 { margin:0 0 12px; }
    .meta { color:#9fb2d1; font-size:14px; }
    .grid { display:grid; grid-template-columns:repeat(4,1fr); gap:16px; margin-top:20px; }
    .card { padding:18px; border-radius:16px; background:#101a2c; border:1px solid rgba(255,255,255,0.08); }
    .value { font-size:28px; font-weight:700; margin-top:8px; }
    .label { color:#90a2c2; font-size:13px; }
    .section { margin-top:24px; padding:20px; border-radius:18px; background:#101a2c; border:1px solid rgba(255,255,255,0.08); }
    table { width:100%; border-collapse:collapse; margin-top:12px; font-size:14px; }
    th,td { text-align:left; padding:10px 12px; border-bottom:1px solid rgba(255,255,255,0.08); }
    th { color:#89a0c7; font-weight:600; }
    ul { margin:12px 0 0; padding-left:18px; }
    li { margin-bottom:12px; color:#c8d8f1; }
    @media (max-width: 900px) { .grid { grid-template-columns:1fr 1fr; } }
    @media (max-width: 640px) { body { padding:16px; } .grid { grid-template-columns:1fr; } }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="hero">
      <h1>AI Singularity Web Report</h1>
      <div class="meta">生成时间：${escapeHtml(generatedAt)}</div>
      <div class="grid">
        <div class="card"><div class="label">今日预估开销</div><div class="value">$${Number(metrics?.total_cost_today_usd ?? 0).toFixed(4)}</div></div>
        <div class="card"><div class="label">活跃 IDE 账号</div><div class="value">${escapeHtml(metrics?.active_ide_accounts ?? 0)}</div></div>
        <div class="card"><div class="label">今日 Token</div><div class="value">${escapeHtml(metrics?.today_total_tokens ?? 0)}</div></div>
        <div class="card"><div class="label">风控阵亡率</div><div class="value">${(((metrics?.forbidden_accounts_ratio ?? 0) as number) * 100).toFixed(1)}%</div></div>
      </div>
    </div>
    <div class="section">
      <h2>公告</h2>
      <ul>${announcementRows || "<li>暂无公告</li>"}</ul>
    </div>
    <div class="section">
      <h2>告警</h2>
      <ul>${alertRows || "<li>暂无告警</li>"}</ul>
    </div>
    <div class="section">
      <h2>Provider 概览</h2>
      <table><thead><tr><th>名称</th><th>平台</th><th>状态</th></tr></thead><tbody>${providerRows || "<tr><td colspan='3'>暂无数据</td></tr>"}</tbody></table>
    </div>
    <div class="section">
      <h2>IDE 账号概览</h2>
      <table><thead><tr><th>账号</th><th>平台</th><th>状态</th><th>项目</th></tr></thead><tbody>${accountRows || "<tr><td colspan='4'>暂无数据</td></tr>"}</tbody></table>
    </div>
    <div class="section">
      <h2>余额摘要</h2>
      <table><thead><tr><th>Provider</th><th>平台</th><th>余额 USD</th><th>剩余额度</th></tr></thead><tbody>${balanceRows || "<tr><td colspan='4'>暂无数据</td></tr>"}</tbody></table>
    </div>
    <div class="section">
      <h2>桌面日志文件</h2>
      <table><thead><tr><th>文件</th><th>类型</th><th>更新时间</th><th>大小</th></tr></thead><tbody>${logRows || "<tr><td colspan='4'>暂无数据</td></tr>"}</tbody></table>
    </div>
  </div>
</body>
</html>`;
}

export default function WebReportPage() {
  const metricsQuery = useQuery({
    queryKey: ["web-report-metrics"],
    queryFn: () => api.analytics.getDashboardMetrics(7),
    staleTime: 30_000,
  });
  const providersQuery = useQuery({
    queryKey: ["web-report-providers"],
    queryFn: () => api.providers.list(),
    staleTime: 30_000,
  });
  const ideAccountsQuery = useQuery({
    queryKey: ["web-report-ide-accounts"],
    queryFn: () => api.ideAccounts.list(),
    staleTime: 30_000,
  });
  const balancesQuery = useQuery({
    queryKey: ["web-report-balances"],
    queryFn: () => api.balance.summaries(),
    staleTime: 30_000,
  });
  const alertsQuery = useQuery({
    queryKey: ["web-report-alerts"],
    queryFn: () => api.alerts.get(),
    staleTime: 30_000,
  });
  const announcementsQuery = useQuery({
    queryKey: ["web-report-announcements"],
    queryFn: () => api.announcements.getState(),
    staleTime: 30_000,
  });
  const logsQuery = useQuery({
    queryKey: ["web-report-logs"],
    queryFn: () => api.logs.list(),
    staleTime: 30_000,
  });

  const loading =
    metricsQuery.isLoading ||
    providersQuery.isLoading ||
    ideAccountsQuery.isLoading ||
    balancesQuery.isLoading ||
    alertsQuery.isLoading ||
    announcementsQuery.isLoading ||
    logsQuery.isLoading;

  const generatedAt = useMemo(() => new Date().toLocaleString(), [
    metricsQuery.dataUpdatedAt,
    providersQuery.dataUpdatedAt,
    ideAccountsQuery.dataUpdatedAt,
    balancesQuery.dataUpdatedAt,
    alertsQuery.dataUpdatedAt,
    announcementsQuery.dataUpdatedAt,
    logsQuery.dataUpdatedAt,
  ]);

  const handleExportHtml = () => {
    const html = buildReportHtml({
      generatedAt,
      metrics: metricsQuery.data,
      providers: providersQuery.data || [],
      ideAccounts: ideAccountsQuery.data || [],
      balances: balancesQuery.data || [],
      alerts: alertsQuery.data || [],
      announcements: announcementsQuery.data?.announcements || [],
      logs: logsQuery.data || [],
    });
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `ai-singularity-web-report-${new Date().toISOString().replace(/[:.]/g, "-")}.html`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <div className="web-report-page">
      <div className="page-header">
        <div>
          <h1 className="page-title"><FileText size={22} className="text-primary" /> Web 报告</h1>
          <p className="page-subtitle">把当前系统状态整理成一份适合导出和归档的浏览器报告。</p>
        </div>
        <div className="web-report-actions">
          <button
            className="btn btn-secondary"
            onClick={() => {
              metricsQuery.refetch();
              providersQuery.refetch();
              ideAccountsQuery.refetch();
              balancesQuery.refetch();
              alertsQuery.refetch();
              announcementsQuery.refetch();
              logsQuery.refetch();
            }}
          >
            <RefreshCw size={14} className={loading ? "spin" : ""} /> 刷新报告数据
          </button>
          <button className="btn btn-primary" onClick={handleExportHtml} disabled={loading}>
            <Download size={14} /> 导出 HTML 报告
          </button>
        </div>
      </div>

      <div className="web-report-meta">
        <span>报告生成时间：{generatedAt}</span>
        <span>数据源：统计、账号、Provider、公告、告警、日志</span>
      </div>

      <div className="web-report-stats">
        <div className="card web-report-stat">
          <div className="web-report-stat-label">今日预估开销</div>
          <div className="web-report-stat-value">
            ${Number(metricsQuery.data?.total_cost_today_usd ?? 0).toFixed(4)}
          </div>
        </div>
        <div className="card web-report-stat">
          <div className="web-report-stat-label">活跃 IDE 账号</div>
          <div className="web-report-stat-value">{ideAccountsQuery.data?.filter((item: any) => item.status === "active").length ?? 0}</div>
        </div>
        <div className="card web-report-stat">
          <div className="web-report-stat-label">未读公告</div>
          <div className="web-report-stat-value">{announcementsQuery.data?.unread_ids?.length ?? 0}</div>
        </div>
        <div className="card web-report-stat">
          <div className="web-report-stat-label">桌面日志文件</div>
          <div className="web-report-stat-value">{logsQuery.data?.length ?? 0}</div>
        </div>
      </div>

      <div className="web-report-grid">
        <section className="card web-report-panel">
          <h3>公告</h3>
          <div className="web-report-list">
            {(announcementsQuery.data?.announcements || []).slice(0, 5).map((item) => (
              <div key={item.id} className="web-report-item">
                <div className="web-report-item-title">{item.title}</div>
                <div className="web-report-item-meta">{formatDateTime(item.created_at)}</div>
                <div className="web-report-item-body">{item.summary || item.content || "—"}</div>
              </div>
            ))}
            {!announcementsQuery.data?.announcements?.length && <div className="web-report-empty">暂无公告</div>}
          </div>
        </section>

        <section className="card web-report-panel">
          <h3>告警</h3>
          <div className="web-report-list">
            {(alertsQuery.data || []).slice(0, 6).map((item: any, index: number) => (
              <div key={item.id || index} className="web-report-item">
                <div className="web-report-item-title">{item.title || item.level || "告警"}</div>
                <div className="web-report-item-body">{item.message || item.description || "—"}</div>
              </div>
            ))}
            {!alertsQuery.data?.length && <div className="web-report-empty">暂无告警</div>}
          </div>
        </section>

        <section className="card web-report-panel span-2">
          <h3>Provider 概览</h3>
          <div className="web-report-table">
            <table>
              <thead>
                <tr>
                  <th>名称</th>
                  <th>平台</th>
                  <th>激活</th>
                  <th>默认模型</th>
                </tr>
              </thead>
              <tbody>
                {(providersQuery.data || []).map((item) => (
                  <tr key={item.id}>
                    <td>{item.name}</td>
                    <td>{item.platform}</td>
                    <td>{item.is_active ? "是" : "否"}</td>
                    <td>{item.model_name}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="card web-report-panel span-2">
          <h3>IDE 账号概览</h3>
          <div className="web-report-table">
            <table>
              <thead>
                <tr>
                  <th>账号</th>
                  <th>平台</th>
                  <th>状态</th>
                  <th>项目</th>
                  <th>更新时间</th>
                </tr>
              </thead>
              <tbody>
                {(ideAccountsQuery.data || []).slice(0, 18).map((item: any) => (
                  <tr key={item.id}>
                    <td>{item.label || item.email}</td>
                    <td>{item.origin_platform}</td>
                    <td>{item.status}</td>
                    <td>{item.project_id || "—"}</td>
                    <td>{formatDateTime(item.updated_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="card web-report-panel">
          <h3>余额摘要</h3>
          <div className="web-report-table compact">
            <table>
              <thead>
                <tr>
                  <th>Provider</th>
                  <th>USD</th>
                  <th>额度</th>
                </tr>
              </thead>
              <tbody>
                {(balancesQuery.data || []).slice(0, 10).map((item) => (
                  <tr key={item.provider_id}>
                    <td>{item.provider_name}</td>
                    <td>{item.latest_balance_usd ?? "—"}</td>
                    <td>{item.quota_remaining ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="card web-report-panel">
          <h3>桌面日志文件</h3>
          <div className="web-report-table compact">
            <table>
              <thead>
                <tr>
                  <th>文件</th>
                  <th>类型</th>
                  <th>更新时间</th>
                </tr>
              </thead>
              <tbody>
                {(logsQuery.data || []).slice(0, 10).map((item) => (
                  <tr key={item.name}>
                    <td>{item.name}</td>
                    <td>{item.kind}</td>
                    <td>{formatDateTime(item.modified_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </div>
  );
}
