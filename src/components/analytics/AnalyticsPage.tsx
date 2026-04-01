import { useEffect, useState, useCallback } from "react";
import { api } from "../../lib/api";
import type { BalanceSummary } from "../../types";
import "./AnalyticsPage.css";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

interface HistoryPoint {
  provider_id: string;
  provider_name: string;
  balance_usd?: number;
  balance_cny?: number;
  quota_remaining?: number;
  quota_unit?: string;
  snapped_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Page
// ─────────────────────────────────────────────────────────────────────────────

export default function AnalyticsPage() {
  const [summaries, setSummaries] = useState<BalanceSummary[]>([]);
  const [history, setHistory] = useState<HistoryPoint[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState("");
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);

  const loadSummaries = useCallback(async () => {
    setIsLoading(true);
    setError("");
    try {
      const data = await api.balance.summaries();
      setSummaries(data);
      if (data.length > 0 && !selectedProvider) {
        setSelectedProvider(data[0].provider_id);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [selectedProvider]);

  const loadHistory = useCallback(async (providerId: string) => {
    try {
      const data = await api.balance.history(providerId, 30);
      setHistory(data.reverse()); // 按时间正序
    } catch (e) {
      console.warn("加载历史失败:", e);
    }
  }, []);

  const handleRefreshAll = async () => {
    setIsRefreshing(true);
    try {
      await api.balance.refreshProviders();
      await loadSummaries();
      setLastRefresh(new Date());
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleRefreshOne = async (providerId: string) => {
    try {
      await api.balance.refreshProvider(providerId);
      await loadSummaries();
      setLastRefresh(new Date());
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { loadSummaries(); }, []);

  useEffect(() => {
    if (selectedProvider) loadHistory(selectedProvider);
  }, [selectedProvider, loadHistory]);

  // ── 衍生统计 ─────────────────────────────────────────────────────────────

  const totalUsd  = summaries.reduce((s, p) => s + (p.latest_balance_usd ?? 0), 0);
  const totalCny  = summaries.reduce((s, p) => s + (p.latest_balance_cny ?? 0), 0);
  const alertCount = summaries.filter(p => p.low_balance_alert).length;
  const activeCount = summaries.length;

  const selectedSummary = summaries.find(s => s.provider_id === selectedProvider);

  return (
    <div className="analytics-page">
      {/* ── Header ────────────────────────────────────────────────────────── */}
      <div className="page-header">
        <div>
          <h1 className="page-title">余额看板</h1>
          <p className="page-subtitle">实时查询各 Provider 余额，追踪使用趋势</p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "center" }}>
          {lastRefresh && (
            <span className="text-muted" style={{ fontSize: 12 }}>
              上次更新：{lastRefresh.toLocaleTimeString()}
            </span>
          )}
          <button
            className="btn btn-primary"
            onClick={handleRefreshAll}
            disabled={isRefreshing}
          >
            {isRefreshing ? "查询中…" : "⟳ 刷新全部"}
          </button>
        </div>
      </div>

      {error && <div className="form-error" style={{ margin: "0 var(--space-8) var(--space-4)" }}>⚠ {error}</div>}

      <div className="analytics-body">
        {/* ── 顶部统计卡片 ─────────────────────────────────────────────── */}
        <div className="stats-row">
          <StatCard
            icon="💰"
            label="USD 余额合计"
            value={totalUsd > 0 ? `$${totalUsd.toFixed(2)}` : "—"}
            accent="var(--color-success)"
          />
          <StatCard
            icon="¥"
            label="CNY 余额合计"
            value={totalCny > 0 ? `¥${totalCny.toFixed(2)}` : "—"}
            accent="var(--color-accent)"
          />
          <StatCard
            icon="🔗"
            label="已配置 Provider"
            value={String(activeCount)}
            accent="var(--color-text-secondary)"
          />
          <StatCard
            icon="⚠"
            label="低余额告警"
            value={String(alertCount)}
            accent={alertCount > 0 ? "var(--color-warning)" : "var(--color-success)"}
            highlight={alertCount > 0}
          />
        </div>

        {/* ── 主体：左侧 Provider 列表 + 右侧详情 ──────────────────────── */}
        {isLoading && summaries.length === 0 ? (
          <div className="empty-state">
            <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
            <span>加载中…</span>
          </div>
        ) : summaries.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📊</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>暂无余额数据</h3>
            <p>先添加 Provider 并绑定 API Key，然后点击「刷新全部」查询余额</p>
            <button className="btn btn-primary" onClick={handleRefreshAll} disabled={isRefreshing}>
              ⟳ 立即查询
            </button>
          </div>
        ) : (
          <div className="analytics-main">
            {/* Provider 列表 */}
            <div className="provider-list-panel">
              <div className="panel-title">Provider 余额</div>
              {summaries.map(s => (
                <ProviderBalanceRow
                  key={s.provider_id}
                  summary={s}
                  isSelected={s.provider_id === selectedProvider}
                  onClick={() => setSelectedProvider(s.provider_id)}
                  onRefresh={() => handleRefreshOne(s.provider_id)}
                />
              ))}
            </div>

            {/* 右侧详情 + 时序图 */}
            <div className="detail-panel">
              {selectedSummary ? (
                <>
                  <ProviderDetail summary={selectedSummary} />
                  <div className="panel-title" style={{ marginTop: "var(--space-4)" }}>
                    余额趋势（最近 30 次）
                  </div>
                  {history.length > 0 ? (
                    <BalanceTrendChart history={history} summary={selectedSummary} />
                  ) : (
                    <div className="empty-chart">
                      <span className="text-muted">暂无历史数据，点击行旁的🔄刷新即可记录第一条</span>
                    </div>
                  )}
                </>
              ) : (
                <div className="empty-chart">
                  <span className="text-muted">← 选择左侧 Provider 查看详情</span>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// StatCard
// ─────────────────────────────────────────────────────────────────────────────

function StatCard({
  icon, label, value, accent, highlight,
}: {
  icon: string; label: string; value: string; accent: string; highlight?: boolean;
}) {
  return (
    <div className={`stat-card card ${highlight ? "stat-card-highlight" : ""}`}>
      <div className="stat-icon" style={{ color: accent }}>{icon}</div>
      <div className="stat-value" style={{ color: accent }}>{value}</div>
      <div className="stat-label text-muted">{label}</div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// ProviderBalanceRow
// ─────────────────────────────────────────────────────────────────────────────

function ProviderBalanceRow({
  summary, isSelected, onClick, onRefresh,
}: {
  summary: BalanceSummary;
  isSelected: boolean;
  onClick: () => void;
  onRefresh: () => void;
}) {
  const mainBalance = summary.latest_balance_usd != null
    ? `$${summary.latest_balance_usd.toFixed(2)}`
    : summary.latest_balance_cny != null
      ? `¥${summary.latest_balance_cny.toFixed(2)}`
      : "—";

  const isLow = summary.low_balance_alert;

  return (
    <div
      className={`provider-balance-row ${isSelected ? "selected" : ""} ${isLow ? "low-balance" : ""}`}
      onClick={onClick}
    >
      <div className="row-main">
        <div className="row-name">{summary.provider_name}</div>
        <div className={`row-balance ${isLow ? "balance-low" : "balance-ok"}`}>{mainBalance}</div>
      </div>
      <div className="row-meta">
        <span className="text-muted" style={{ fontSize: 11 }}>
          {summary.last_updated
            ? `更新于 ${new Date(summary.last_updated).toLocaleTimeString()}`
            : "未查询"}
        </span>
        {isLow && <span className="badge-warning">余额低</span>}
      </div>
      <button
        className="refresh-btn"
        onClick={e => { e.stopPropagation(); onRefresh(); }}
        title="刷新余额"
      >
        🔄
      </button>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// ProviderDetail
// ─────────────────────────────────────────────────────────────────────────────

function ProviderDetail({ summary }: { summary: BalanceSummary }) {
  return (
    <div className="provider-detail-card card">
      <div className="detail-header">
        <h3>{summary.provider_name}</h3>
        {summary.low_balance_alert && (
          <span className="badge-warning">⚠ 余额偏低</span>
        )}
      </div>
      <div className="detail-grid">
        {summary.latest_balance_usd != null && (
          <DetailItem label="USD 余额" value={`$${summary.latest_balance_usd.toFixed(4)}`} />
        )}
        {summary.latest_balance_cny != null && (
          <DetailItem label="CNY 余额" value={`¥${summary.latest_balance_cny.toFixed(2)}`} />
        )}
        {summary.quota_remaining != null && (
          <DetailItem
            label={`配额剩余 (${summary.quota_unit ?? ""})`}
            value={summary.quota_remaining.toFixed(2)}
          />
        )}
        {summary.quota_reset_at && (
          <DetailItem
            label="配额重置"
            value={new Date(summary.quota_reset_at).toLocaleDateString()}
          />
        )}
        {summary.last_updated && (
          <DetailItem
            label="最后更新"
            value={new Date(summary.last_updated).toLocaleString()}
          />
        )}
        <DetailItem label="平台" value={summary.platform} />
      </div>
    </div>
  );
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-item">
      <span className="text-muted">{label}</span>
      <span className="font-mono">{value}</span>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// BalanceTrendChart（纯 CSS 折线图，无需第三方图表库）
// ─────────────────────────────────────────────────────────────────────────────

function BalanceTrendChart({
  history,
  summary,
}: {
  history: HistoryPoint[];
  summary: BalanceSummary;
}) {
  if (history.length === 0) return null;

  // 使用 USD 或 CNY（优先 USD）
  const useUsd = history.some(h => h.balance_usd != null);
  const values = history.map(h => (useUsd ? h.balance_usd : h.balance_cny) ?? 0);
  const unit = useUsd ? "USD" : "CNY";
  const prefix = useUsd ? "$" : "¥";

  const maxVal = Math.max(...values, 0.001);
  const minVal = Math.min(...values, 0);

  const W = 100; // SVG viewBox width %
  const H = 100; // SVG viewBox height %
  const PAD = 8;

  // 归一化到 [PAD, H-PAD]
  const normalize = (v: number) => {
    const range = maxVal - minVal || 1;
    return H - PAD - ((v - minVal) / range) * (H - 2 * PAD);
  };

  const points = values.map((v, i) => {
    const x = PAD + (i / Math.max(values.length - 1, 1)) * (W - 2 * PAD);
    const y = normalize(v);
    return `${x.toFixed(2)},${y.toFixed(2)}`;
  }).join(" ");

  const lastValue = values[values.length - 1] ?? 0;
  const firstValue = values[0] ?? 0;
  const trend = lastValue >= firstValue ? "↑" : "↓";
  const trendColor = lastValue >= firstValue ? "var(--color-success)" : "var(--color-error)";

  return (
    <div className="trend-chart-container">
      <div className="chart-header">
        <span className="text-muted" style={{ fontSize: 12 }}>余额 ({unit})</span>
        <span style={{ fontSize: 13, color: trendColor, fontWeight: 600 }}>
          {trend} {prefix}{lastValue.toFixed(useUsd ? 4 : 2)}
        </span>
      </div>

      <svg viewBox={`0 0 ${W} ${H}`} className="trend-svg" preserveAspectRatio="none">
        {/* 背景网格 */}
        {[0.25, 0.5, 0.75].map(f => (
          <line
            key={f}
            x1={PAD} y1={(f * (H - 2 * PAD) + PAD).toFixed(1)}
            x2={W - PAD} y2={(f * (H - 2 * PAD) + PAD).toFixed(1)}
            stroke="var(--color-border)"
            strokeWidth="0.5"
            strokeDasharray="2,2"
          />
        ))}
        {/* 填充区域 */}
        <polygon
          points={`${PAD},${H - PAD} ${points} ${W - PAD},${H - PAD}`}
          fill="url(#grad)"
          opacity="0.2"
        />
        {/* 折线 */}
        <polyline
          points={points}
          fill="none"
          stroke="var(--color-accent)"
          strokeWidth="1.5"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
        {/* 最后一个点圆心 */}
        {values.length > 0 && (() => {
          const last = points.split(" ").pop()!.split(",");
          return (
            <circle cx={last[0]} cy={last[1]} r="2.5" fill="var(--color-accent)" />
          );
        })()}
        <defs>
          <linearGradient id="grad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="0.8" />
            <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
      </svg>

      {/* X 轴时间标签 */}
      <div className="chart-footer">
        <span className="text-muted">{history.length > 0 ? new Date(history[0].snapped_at).toLocaleDateString() : ""}</span>
        <span className="text-muted">{history.length > 0 ? new Date(history[history.length - 1].snapped_at).toLocaleDateString() : ""}</span>
      </div>
    </div>
  );
}
