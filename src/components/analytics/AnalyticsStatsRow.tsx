type AnalyticsStatsRowProps = {
  totalUsd: number;
  totalCny: number;
  activeCount: number;
  alertCount: number;
};

function StatCard({
  icon,
  label,
  value,
  accent,
  highlight,
}: {
  icon: string;
  label: string;
  value: string;
  accent: string;
  highlight?: boolean;
}) {
  return (
    <div className={`stat-card card ${highlight ? "stat-card-highlight" : ""}`}>
      <div className="stat-icon" style={{ color: accent }}>{icon}</div>
      <div className="stat-value" style={{ color: accent }}>{value}</div>
      <div className="stat-label text-muted">{label}</div>
    </div>
  );
}

export function AnalyticsStatsRow({
  totalUsd,
  totalCny,
  activeCount,
  alertCount,
}: AnalyticsStatsRowProps) {
  return (
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
  );
}
