import { useEffect, useMemo, useState } from "react";
import { api } from "../../lib/api";
import { PLATFORM_LABELS, type Model, type Platform } from "../../types";
import "./ModelsPage.css";

function formatContextLength(value?: number) {
  if (!value) return "未知";
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(value % 1_000_000 === 0 ? 0 : 1)}M`;
  if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
  return String(value);
}

function formatPrice(value?: number) {
  return value == null ? "—" : `$${value.toFixed(2)}`;
}

export default function ModelsPage() {
  const [models, setModels] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [platform, setPlatform] = useState<Platform | "all">("all");

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setLoading(true);
      setError("");
      try {
        const data = await api.models.list();
        if (!cancelled) setModels(data);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const platformOptions = useMemo(() => {
    return Array.from(new Set(models.map((item) => item.platform))).sort();
  }, [models]);

  const filteredModels = useMemo(() => {
    const q = search.trim().toLowerCase();
    return models
      .filter((item) => platform === "all" || item.platform === platform)
      .filter((item) => {
        if (!q) return true;
        return (
          item.name.toLowerCase().includes(q) ||
          item.id.toLowerCase().includes(q) ||
          PLATFORM_LABELS[item.platform].toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        const platformCompare = PLATFORM_LABELS[a.platform].localeCompare(PLATFORM_LABELS[b.platform]);
        return platformCompare !== 0 ? platformCompare : a.name.localeCompare(b.name);
      });
  }, [models, platform, search]);

  const stats = useMemo(() => {
    const totalPlatforms = new Set(filteredModels.map((item) => item.platform)).size;
    const visionCount = filteredModels.filter((item) => item.supports_vision).length;
    const toolsCount = filteredModels.filter((item) => item.supports_tools).length;

    return {
      total: filteredModels.length,
      totalPlatforms,
      visionCount,
      toolsCount,
    };
  }, [filteredModels]);

  return (
    <div className="models-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">模型目录</h1>
          <p className="page-subtitle">本地静态模型目录，先解决能力检索与横向对比</p>
        </div>
      </div>

      <div className="models-body">
        <div className="models-stats">
          <StatCard label="模型总数" value={String(stats.total)} />
          <StatCard label="平台数量" value={String(stats.totalPlatforms)} />
          <StatCard label="支持视觉" value={String(stats.visionCount)} />
          <StatCard label="支持工具" value={String(stats.toolsCount)} />
        </div>

        <div className="card models-toolbar">
          <input
            className="form-input"
            placeholder="搜索模型名、ID 或平台"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <select
            className="form-input"
            value={platform}
            onChange={(e) => setPlatform(e.target.value as Platform | "all")}
          >
            <option value="all">全部平台</option>
            {platformOptions.map((item) => (
              <option key={item} value={item}>
                {PLATFORM_LABELS[item]}
              </option>
            ))}
          </select>
        </div>

        {error && (
          <div className="card" style={{ borderColor: "var(--color-danger)" }}>
            <div className="text-danger">模型目录加载失败：{error}</div>
          </div>
        )}

        {loading ? (
          <div className="empty-state">
            <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
            <span>加载模型目录中...</span>
          </div>
        ) : filteredModels.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">🔎</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>没有匹配的模型</h3>
            <p>试试调整搜索词或平台筛选。</p>
          </div>
        ) : (
          <div className="models-grid">
            {filteredModels.map((item) => (
              <article key={`${item.platform}-${item.id}`} className="card model-card">
                <div className="model-card-header">
                  <div>
                    <h3 className="model-card-title">{item.name}</h3>
                    <div className="model-card-id">{item.id}</div>
                  </div>
                  <span className="model-platform-chip">{PLATFORM_LABELS[item.platform]}</span>
                </div>

                <div className="model-capabilities">
                  <span className={`cap-chip ${item.supports_vision ? "enabled" : "disabled"}`}>
                    视觉 {item.supports_vision ? "支持" : "不支持"}
                  </span>
                  <span className={`cap-chip ${item.supports_tools ? "enabled" : "disabled"}`}>
                    工具 {item.supports_tools ? "支持" : "不支持"}
                  </span>
                </div>

                <div className="model-metrics">
                  <MetricItem label="上下文" value={formatContextLength(item.context_length)} />
                  <MetricItem label="输入价 / 1M" value={formatPrice(item.input_price_per_1m)} />
                  <MetricItem label="输出价 / 1M" value={formatPrice(item.output_price_per_1m)} />
                  <MetricItem label="状态" value={item.is_available ? "可用" : "未知"} />
                </div>
              </article>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="card models-stat-card">
      <div className="models-stat-value">{value}</div>
      <div className="models-stat-label">{label}</div>
    </div>
  );
}

function MetricItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="model-metric-item">
      <span className="text-muted">{label}</span>
      <span className="font-mono">{value}</span>
    </div>
  );
}
