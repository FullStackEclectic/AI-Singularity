import type { BalanceSummary } from "../../types";
import type { HistoryPoint } from "./analyticsTypes";

type AnalyticsProviderDetailPanelProps = {
  selectedSummary?: BalanceSummary;
  history: HistoryPoint[];
};

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-item">
      <span className="text-muted">{label}</span>
      <span className="font-mono">{value}</span>
    </div>
  );
}

function ProviderDetail({ summary }: { summary: BalanceSummary }) {
  return (
    <div className="provider-detail-card card">
      <div className="detail-header">
        <h3>{summary.provider_name}</h3>
        {summary.low_balance_alert && <span className="badge-warning">⚠ 余额偏低</span>}
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

function BalanceTrendChart({ history }: { history: HistoryPoint[] }) {
  if (history.length === 0) return null;

  const useUsd = history.some((item) => item.balance_usd != null);
  const values = history.map((item) => (useUsd ? item.balance_usd : item.balance_cny) ?? 0);
  const unit = useUsd ? "USD" : "CNY";
  const prefix = useUsd ? "$" : "¥";

  const maxVal = Math.max(...values, 0.001);
  const minVal = Math.min(...values, 0);

  const width = 100;
  const height = 100;
  const pad = 8;

  const normalize = (value: number) => {
    const range = maxVal - minVal || 1;
    return height - pad - ((value - minVal) / range) * (height - 2 * pad);
  };

  const points = values.map((value, index) => {
    const x = pad + (index / Math.max(values.length - 1, 1)) * (width - 2 * pad);
    const y = normalize(value);
    return `${x.toFixed(2)},${y.toFixed(2)}`;
  }).join(" ");

  const lastValue = values[values.length - 1] ?? 0;
  const firstValue = values[0] ?? 0;
  const trendUp = lastValue >= firstValue;

  return (
    <div className="trend-chart-container">
      <div className="chart-header">
        <span className="text-muted" style={{ fontSize: 12 }}>余额 ({unit})</span>
        <span
          style={{
            fontSize: 13,
            color: trendUp ? "var(--color-success)" : "var(--color-error)",
            fontWeight: 600,
          }}
        >
          {trendUp ? "↑" : "↓"} {prefix}{lastValue.toFixed(useUsd ? 4 : 2)}
        </span>
      </div>

      <svg viewBox={`0 0 ${width} ${height}`} className="trend-svg" preserveAspectRatio="none">
        {[0.25, 0.5, 0.75].map((fraction) => (
          <line
            key={fraction}
            x1={pad}
            y1={(fraction * (height - 2 * pad) + pad).toFixed(1)}
            x2={width - pad}
            y2={(fraction * (height - 2 * pad) + pad).toFixed(1)}
            stroke="var(--color-border)"
            strokeWidth="0.5"
            strokeDasharray="2,2"
          />
        ))}
        <polygon
          points={`${pad},${height - pad} ${points} ${width - pad},${height - pad}`}
          fill="url(#grad)"
          opacity="0.2"
        />
        <polyline
          points={points}
          fill="none"
          stroke="var(--color-accent)"
          strokeWidth="1.5"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
        {values.length > 0 && (() => {
          const lastPoint = points.split(" ").pop()?.split(",") ?? [];
          return <circle cx={lastPoint[0]} cy={lastPoint[1]} r="2.5" fill="var(--color-accent)" />;
        })()}
        <defs>
          <linearGradient id="grad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="0.8" />
            <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
      </svg>

      <div className="chart-footer">
        <span className="text-muted">
          {history.length > 0 ? new Date(history[0].snapped_at).toLocaleDateString() : ""}
        </span>
        <span className="text-muted">
          {history.length > 0 ? new Date(history[history.length - 1].snapped_at).toLocaleDateString() : ""}
        </span>
      </div>
    </div>
  );
}

export function AnalyticsProviderDetailPanel({
  selectedSummary,
  history,
}: AnalyticsProviderDetailPanelProps) {
  return (
    <div className="detail-panel">
      {selectedSummary ? (
        <>
          <ProviderDetail summary={selectedSummary} />
          <div className="panel-title" style={{ marginTop: "var(--space-4)" }}>
            余额趋势（最近 30 次）
          </div>
          {history.length > 0 ? (
            <BalanceTrendChart history={history} />
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
  );
}
