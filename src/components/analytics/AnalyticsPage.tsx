import { useEffect, useState, useCallback } from "react";
import { api } from "../../lib/api";
import type { BalanceSummary, TokenUsageStat } from "../../types";
import { AnalyticsProviderDetailPanel } from "./AnalyticsProviderDetailPanel";
import { AnalyticsProviderList } from "./AnalyticsProviderList";
import { AnalyticsStatsRow } from "./AnalyticsStatsRow";
import { AnalyticsTokenUsageSection } from "./AnalyticsTokenUsageSection";
import type { HistoryPoint } from "./analyticsTypes";
import "./AnalyticsPage.css";

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
  const [tokenByApp, setTokenByApp] = useState<TokenUsageStat[]>([]);
  const [tokenByModel, setTokenByModel] = useState<TokenUsageStat[]>([]);

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
    // 加载 Token 用量统计数据
    api.stats.getTokenUsage().then((data) => {
      setTokenByApp(data.by_app ?? []);
      setTokenByModel(data.by_model ?? []);
    }).catch(() => {});
  }, [lastRefresh]); // 每次刷新后重载

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
        <AnalyticsStatsRow
          totalUsd={totalUsd}
          totalCny={totalCny}
          activeCount={activeCount}
          alertCount={alertCount}
        />

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
            <AnalyticsProviderList
              summaries={summaries}
              selectedProvider={selectedProvider}
              onSelectProvider={setSelectedProvider}
              onRefreshProvider={handleRefreshOne}
            />
            <AnalyticsProviderDetailPanel
              selectedSummary={selectedSummary}
              history={history}
            />
          </div>
        )}
      </div>
      <AnalyticsTokenUsageSection tokenByApp={tokenByApp} tokenByModel={tokenByModel} />
    </div>
  );
}
