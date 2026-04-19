import type { BalanceSummary } from "../../types";

type AnalyticsProviderListProps = {
  summaries: BalanceSummary[];
  selectedProvider: string | null;
  onSelectProvider: (providerId: string) => void;
  onRefreshProvider: (providerId: string) => void;
};

function ProviderBalanceRow({
  summary,
  isSelected,
  onClick,
  onRefresh,
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
        onClick={(event) => {
          event.stopPropagation();
          onRefresh();
        }}
        title="刷新余额"
      >
        🔄
      </button>
    </div>
  );
}

export function AnalyticsProviderList({
  summaries,
  selectedProvider,
  onSelectProvider,
  onRefreshProvider,
}: AnalyticsProviderListProps) {
  return (
    <div className="provider-list-panel">
      <div className="panel-title">Provider 余额</div>
      {summaries.map((summary) => (
        <ProviderBalanceRow
          key={summary.provider_id}
          summary={summary}
          isSelected={summary.provider_id === selectedProvider}
          onClick={() => onSelectProvider(summary.provider_id)}
          onRefresh={() => onRefreshProvider(summary.provider_id)}
        />
      ))}
    </div>
  );
}
