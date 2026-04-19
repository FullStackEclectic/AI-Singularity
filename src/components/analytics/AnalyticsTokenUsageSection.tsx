import type { TokenUsageStat } from "../../types";

const PIE_COLORS = [
  "var(--color-accent)",
  "#22d3ee",
  "#a78bfa",
  "#fb923c",
  "#4ade80",
  "#f472b6",
  "#facc15",
];

function TokenPieChart({ data }: { data: TokenUsageStat[] }) {
  if (data.length === 0) return null;
  const total = data.reduce((sum, item) => sum + item.total_tokens, 0) || 1;

  let cumulativeAngle = -Math.PI / 2;
  const cx = 80;
  const cy = 80;
  const r = 70;

  const slices = data.map((item, index) => {
    const ratio = item.total_tokens / total;
    const angle = ratio * 2 * Math.PI;
    const startAngle = cumulativeAngle;
    const endAngle = cumulativeAngle + angle;
    cumulativeAngle = endAngle;

    const x1 = cx + r * Math.cos(startAngle);
    const y1 = cy + r * Math.sin(startAngle);
    const x2 = cx + r * Math.cos(endAngle);
    const y2 = cy + r * Math.sin(endAngle);
    const largeArc = angle > Math.PI ? 1 : 0;

    return {
      path: `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`,
      color: PIE_COLORS[index % PIE_COLORS.length],
      label: item.name,
      ratio,
      tokens: item.total_tokens,
    };
  });

  return (
    <div className="token-pie-container">
      <svg viewBox="0 0 160 160" width="160" height="160">
        {slices.map((slice, index) => (
          <path
            key={index}
            d={slice.path}
            fill={slice.color}
            opacity={0.9}
            stroke="var(--color-surface)"
            strokeWidth={1}
          >
            <title>{slice.label}: {slice.tokens.toLocaleString()} tokens ({(slice.ratio * 100).toFixed(1)}%)</title>
          </path>
        ))}
        <circle cx={cx} cy={cy} r={32} fill="var(--color-surface)" />
        <text x={cx} y={cy - 4} textAnchor="middle" fontSize={10} fill="var(--color-text-muted)">Total</text>
        <text x={cx} y={cy + 10} textAnchor="middle" fontSize={9} fill="var(--color-text-primary)">
          {total >= 1000000
            ? `${(total / 1000000).toFixed(1)}M`
            : total >= 1000
              ? `${(total / 1000).toFixed(0)}K`
              : total}
        </text>
      </svg>
      <div className="pie-legend">
        {slices.map((slice, index) => (
          <div key={index} className="pie-legend-item">
            <span className="pie-legend-dot" style={{ background: slice.color }} />
            <span className="pie-legend-name text-muted">{slice.label}</span>
            <span className="pie-legend-pct">{(slice.ratio * 100).toFixed(1)}%</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function TokenBarChart({ data }: { data: TokenUsageStat[] }) {
  if (data.length === 0) return null;
  const sorted = [...data].sort((a, b) => b.total_tokens - a.total_tokens).slice(0, 8);
  const maxVal = sorted[0]?.total_tokens || 1;

  return (
    <div className="token-bar-container">
      {sorted.map((item, index) => {
        const ratio = item.total_tokens / maxVal;
        return (
          <div key={index} className="token-bar-row">
            <div className="token-bar-label text-muted" title={item.name}>
              {item.name.length > 22 ? `${item.name.slice(0, 20)}…` : item.name}
            </div>
            <div className="token-bar-track">
              <div
                className="token-bar-fill"
                style={{
                  width: `${ratio * 100}%`,
                  background: PIE_COLORS[index % PIE_COLORS.length],
                }}
              />
            </div>
            <div className="token-bar-value">
              {item.total_tokens >= 1000000
                ? `${(item.total_tokens / 1000000).toFixed(2)}M`
                : item.total_tokens >= 1000
                  ? `${(item.total_tokens / 1000).toFixed(1)}K`
                  : item.total_tokens}
            </div>
          </div>
        );
      })}
    </div>
  );
}

type AnalyticsTokenUsageSectionProps = {
  tokenByApp: TokenUsageStat[];
  tokenByModel: TokenUsageStat[];
};

export function AnalyticsTokenUsageSection({
  tokenByApp,
  tokenByModel,
}: AnalyticsTokenUsageSectionProps) {
  if (tokenByApp.length === 0 && tokenByModel.length === 0) {
    return null;
  }

  return (
    <div className="analytics-token-section">
      <div className="section-divider">
        <span className="section-divider-label">Token 用量分析</span>
      </div>
      <div className="token-charts-row">
        {tokenByApp.length > 0 && (
          <div className="token-chart-card card">
            <div className="panel-title">应用占比（按工具）</div>
            <TokenPieChart data={tokenByApp} />
          </div>
        )}
        {tokenByModel.length > 0 && (
          <div className="token-chart-card card">
            <div className="panel-title">模型用量对比</div>
            <TokenBarChart data={tokenByModel} />
          </div>
        )}
      </div>
    </div>
  );
}
