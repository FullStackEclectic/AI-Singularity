import { useEffect, useMemo, useState } from "react";
import { api } from "../../lib/api";
import { PLATFORM_LABELS, type Model, type Platform } from "../../types";
import "./ModelsPage.css";

type PricingDraft = {
  fixed: string;
  request: string;
  input: string;
  output: string;
  currency: string;
  unit: string;
  note: string;
};

type PricingSourceFilter = "all" | "unset" | "builtin" | "manual" | "special";
type PricingCurrencyFilter = "all" | "USD" | "CNY" | "other";
type BillingShapeFilter = "all" | "token" | "hybrid" | "fixed" | "unknown";

function formatContextLength(value?: number) {
  if (!value) return "未知";
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(value % 1_000_000 === 0 ? 0 : 1)}M`;
  if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
  return String(value);
}

function formatPricingUnit(value?: string) {
  switch (value) {
    case "1m_tokens":
      return "1M token";
    case "request":
      return "每次请求";
    case "image":
      return "每张图像";
    default:
      return value || "—";
  }
}

function formatPricingCurrency(value?: string) {
  if (!value) return "—";
  if (value === "USD") return "美元";
  if (value === "CNY") return "人民币";
  return value;
}

function formatPrice(value?: number, currency?: string, unit?: string) {
  if (value == null) return "待设置";
  const prefix = currency === "CNY" ? "¥" : currency === "USD" ? "$" : currency ? `${currency} ` : "";
  const suffix = unit ? ` / ${formatPricingUnit(unit)}` : "";
  return `${prefix}${value.toFixed(4)}${suffix}`;
}

function parsePriceInput(value: string): number | undefined {
  const normalized = value.trim();
  if (!normalized) return undefined;
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function isFixedPricingUnit(unit?: string) {
  return unit === "request" || unit === "image";
}

function formatPricingSource(model: Model) {
  switch (model.pricing_source) {
    case "manual":
      return "手工覆盖";
    case "builtin":
      return "内置基线";
    case "special":
      return "特殊口径";
    default:
      return "待补充";
  }
}

function formatPricingUpdatedAt(value?: string) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function getBillingShape(model: Model) {
  const unit = model.pricing_unit || model.base_pricing_unit;
  const hasFixedPrice = model.fixed_price != null || model.base_fixed_price != null;
  const hasTokenPrice =
    model.input_price_per_1m != null ||
    model.output_price_per_1m != null ||
    model.base_input_price_per_1m != null ||
    model.base_output_price_per_1m != null;

  if (isFixedPricingUnit(unit) || (hasFixedPrice && !hasTokenPrice)) {
    return "fixed";
  }
  if (model.request_price != null || model.base_request_price != null) {
    return "hybrid";
  }
  if (hasTokenPrice) {
    return "token";
  }
  return "unknown";
}

function formatBillingShape(model: Model) {
  switch (getBillingShape(model)) {
    case "fixed":
      return (model.pricing_unit || model.base_pricing_unit) === "image" ? "按图像固定价" : "固定单价";
    case "hybrid":
      return "Token + 请求费";
    case "token":
      return "按 Token";
    default:
      return "待补充";
  }
}

function buildModelKey(model: Model) {
  return `${model.platform}::${model.id}`;
}

function createDraft(model: Model): PricingDraft {
  return {
    fixed: model.fixed_price?.toString() || "",
    request: model.request_price?.toString() || "",
    input: model.input_price_per_1m?.toString() || "",
    output: model.output_price_per_1m?.toString() || "",
    currency: model.pricing_currency || model.base_pricing_currency || "USD",
    unit: model.pricing_unit || model.base_pricing_unit || "1m_tokens",
    note: model.pricing_note || "",
  };
}

export default function ModelsPage() {
  const [models, setModels] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [search, setSearch] = useState("");
  const [platform, setPlatform] = useState<Platform | "all">("all");
  const [sourceFilter, setSourceFilter] = useState<PricingSourceFilter>("all");
  const [currencyFilter, setCurrencyFilter] = useState<PricingCurrencyFilter>("all");
  const [billingFilter, setBillingFilter] = useState<BillingShapeFilter>("all");
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, PricingDraft>>({});
  const [savingKey, setSavingKey] = useState<string | null>(null);

  const loadModels = async () => {
    setLoading(true);
    setError("");
    try {
      const data = await api.models.list();
      setModels(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadModels();
  }, []);

  const platformOptions = useMemo(() => {
    return Array.from(new Set(models.map((item) => item.platform))).sort();
  }, [models]);

  const filteredModels = useMemo(() => {
    const q = search.trim().toLowerCase();
    return models
      .filter((item) => platform === "all" || item.platform === platform)
      .filter((item) => sourceFilter === "all" || (item.pricing_source || "unset") === sourceFilter)
      .filter((item) => billingFilter === "all" || getBillingShape(item) === billingFilter)
      .filter((item) => {
        if (currencyFilter === "all") return true;
        const currency = item.pricing_currency || item.base_pricing_currency || "";
        if (currencyFilter === "other") return Boolean(currency) && currency !== "USD" && currency !== "CNY";
        return currency === currencyFilter;
      })
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
  }, [billingFilter, currencyFilter, models, platform, search, sourceFilter]);

  const stats = useMemo(() => {
    const totalPlatforms = new Set(filteredModels.map((item) => item.platform)).size;
    const pricedCount = filteredModels.filter(
      (item) =>
        item.fixed_price != null ||
        item.request_price != null ||
        item.input_price_per_1m != null ||
        item.output_price_per_1m != null
    ).length;
    const manualCount = filteredModels.filter((item) => item.pricing_source === "manual").length;
    const unsetCount = filteredModels.filter((item) => item.pricing_source === "unset").length;
    const specialCount = filteredModels.filter((item) => item.pricing_source === "special").length;
    const usdCount = filteredModels.filter((item) => (item.pricing_currency || item.base_pricing_currency) === "USD").length;
    const cnyCount = filteredModels.filter((item) => (item.pricing_currency || item.base_pricing_currency) === "CNY").length;
    const hybridCount = filteredModels.filter((item) => getBillingShape(item) === "hybrid").length;
    const requestCount = filteredModels.filter((item) => getBillingShape(item) === "fixed").length;

    return {
      total: filteredModels.length,
      totalPlatforms,
      pricedCount,
      manualCount,
      unsetCount,
      specialCount,
      usdCount,
      cnyCount,
      hybridCount,
      requestCount,
    };
  }, [filteredModels]);

  const updateDraft = (key: string, patch: Partial<PricingDraft>) => {
    setDrafts((prev) => ({
      ...prev,
      [key]: (() => {
        const current = prev[key] || {
          fixed: "",
          request: "",
          input: "",
          output: "",
          currency: "USD",
          unit: "1m_tokens",
          note: "",
        };

        if (patch.unit && patch.unit !== current.unit) {
          if (isFixedPricingUnit(patch.unit)) {
            return {
              ...current,
              ...patch,
              request: "",
              input: "",
              output: "",
            };
          }

          return {
            ...current,
            ...patch,
            fixed: "",
          };
        }

        return {
          ...current,
          ...patch,
        };
      })(),
    }));
  };

  const beginEdit = (model: Model) => {
    const key = buildModelKey(model);
    setMessage("");
    setEditingKey(key);
    setDrafts((prev) => ({
      ...prev,
      [key]: createDraft(model),
    }));
  };

  const cancelEdit = () => {
    setEditingKey(null);
  };

  const handleSave = async (model: Model) => {
    const key = buildModelKey(model);
    const draft = drafts[key] || createDraft(model);
    setSavingKey(key);
    setError("");
    setMessage("");
    try {
      const fixedMode = isFixedPricingUnit(draft.unit);
      await api.models.savePricing({
        platform: model.platform,
        modelId: model.id,
        fixedPrice: fixedMode ? parsePriceInput(draft.fixed) : undefined,
        requestPrice: fixedMode ? undefined : parsePriceInput(draft.request),
        inputPricePer1m: fixedMode ? undefined : parsePriceInput(draft.input),
        outputPricePer1m: fixedMode ? undefined : parsePriceInput(draft.output),
        pricingCurrency: draft.currency || undefined,
        pricingUnit: draft.unit || undefined,
        note: draft.note.trim() || undefined,
      });
      await loadModels();
      setEditingKey(null);
      setMessage(`已更新 ${model.name} 的基础价格。`);
    } catch (e) {
      setError(String(e));
    } finally {
      setSavingKey(null);
    }
  };

  const handleReset = async (model: Model) => {
    const key = buildModelKey(model);
    setSavingKey(key);
    setError("");
    setMessage("");
    try {
      await api.models.resetPricing({
        platform: model.platform,
        modelId: model.id,
      });
      await loadModels();
      setEditingKey(null);
      setMessage(`已恢复 ${model.name} 的内置基础价格。`);
    } catch (e) {
      setError(String(e));
    } finally {
      setSavingKey(null);
    }
  };

  return (
    <div className="models-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">模型目录</h1>
          <p className="page-subtitle">把模型能力速查和基础价格基线收在同一个目录里，供换算与成本统计复用。</p>
        </div>
      </div>

      <div className="models-body">
        <div className="card models-banner models-banner-info">
          <strong>基础价格源优先保留官方原始币种与计费单位。</strong>
          模型目录会展示 USD、CNY 或特殊计费口径；当前成本统计仍只复用 <code>USD / 1M token</code> 与 <code>USD / request</code> 这类可直接换算的单价，其他口径会保留在目录里作基线说明。
          {stats.unsetCount > 0
            ? ` 当前还有 ${stats.unsetCount} 个模型待补充。`
            : ` 当前目录里的主流公开价格档已基本补齐；其中 USD ${stats.usdCount} 个、CNY ${stats.cnyCount} 个、混合计费 ${stats.hybridCount} 个、固定单价 ${stats.requestCount} 个、特殊口径 ${stats.specialCount} 个。`}
        </div>

        <div className="models-stats">
          <StatCard label="模型总数" value={String(stats.total)} />
          <StatCard label="平台数量" value={String(stats.totalPlatforms)} />
          <StatCard label="已配置基础价" value={String(stats.pricedCount)} />
          <StatCard label="USD 口径" value={String(stats.usdCount)} />
          <StatCard label="CNY 口径" value={String(stats.cnyCount)} />
          <StatCard label="混合计费" value={String(stats.hybridCount)} />
          <StatCard label="固定单价" value={String(stats.requestCount)} />
          <StatCard label="特殊口径" value={String(stats.specialCount)} />
          <StatCard label="手工覆盖" value={String(stats.manualCount)} />
          <StatCard label="待补充" value={String(stats.unsetCount)} />
        </div>

        <div className="card models-toolbar">
          <input
            className="form-input"
            placeholder="搜索模型名、ID 或平台"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <div className="models-toolbar-actions">
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
            <select
              className="form-input"
              value={sourceFilter}
              onChange={(e) => setSourceFilter(e.target.value as PricingSourceFilter)}
            >
              <option value="all">全部来源</option>
              <option value="unset">仅待补充</option>
              <option value="builtin">仅内置基线</option>
              <option value="manual">仅手工覆盖</option>
              <option value="special">仅特殊口径</option>
            </select>
            <select
              className="form-input"
              value={currencyFilter}
              onChange={(e) => setCurrencyFilter(e.target.value as PricingCurrencyFilter)}
            >
              <option value="all">全部币种</option>
              <option value="USD">仅 USD</option>
              <option value="CNY">仅 CNY</option>
              <option value="other">其他币种</option>
            </select>
            <select
              className="form-input"
              value={billingFilter}
              onChange={(e) => setBillingFilter(e.target.value as BillingShapeFilter)}
            >
              <option value="all">全部计费形态</option>
              <option value="token">仅按 Token</option>
              <option value="hybrid">仅混合计费</option>
              <option value="fixed">仅固定单价</option>
              <option value="unknown">仅待补充形态</option>
            </select>
            <button className="btn btn-secondary" onClick={() => void loadModels()} disabled={loading}>
              {loading ? "刷新中..." : "刷新"}
            </button>
          </div>
        </div>

        {message && (
          <div className="card models-banner models-banner-success">
            {message}
          </div>
        )}

        {error && (
          <div className="card models-banner models-banner-error">
            模型目录加载失败：{error}
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
            {filteredModels.map((item) => {
              const key = buildModelKey(item);
              const isEditing = editingKey === key;
              const draft = drafts[key] || createDraft(item);
              const isSaving = savingKey === key;
              const currentIsFixed = isFixedPricingUnit(item.pricing_unit || item.base_pricing_unit);
              const draftIsFixed = isFixedPricingUnit(draft.unit);

              return (
                <article key={key} className="card model-card">
                  <div className="model-card-header">
                    <div>
                      <h3 className="model-card-title">{item.name}</h3>
                      <div className="model-card-id">{item.id}</div>
                    </div>
                    <span className="model-platform-chip">{PLATFORM_LABELS[item.platform]}</span>
                  </div>

                  <div className="model-meta-row">
                    <div className="model-meta-chips">
                      <span className={`model-source-chip ${item.pricing_source || "unset"}`}>
                        {formatPricingSource(item)}
                      </span>
                      <span className={`model-billing-chip ${getBillingShape(item)}`}>
                        {formatBillingShape(item)}
                      </span>
                    </div>
                    <button className="btn btn-ghost btn-sm" onClick={() => beginEdit(item)}>
                      {isEditing ? "编辑中" : "编辑基础价"}
                    </button>
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
                    {currentIsFixed ? (
                      <MetricItem
                        label="当前固定价"
                        value={formatPrice(item.fixed_price, item.pricing_currency, item.pricing_unit)}
                      />
                    ) : (
                      <>
                        <MetricItem
                          label="当前输入价"
                          value={formatPrice(item.input_price_per_1m, item.pricing_currency, item.pricing_unit)}
                        />
                        <MetricItem
                          label="当前输出价"
                          value={formatPrice(item.output_price_per_1m, item.pricing_currency, item.pricing_unit)}
                        />
                        <MetricItem
                          label="当前请求费"
                          value={formatPrice(item.request_price, item.pricing_currency, "request")}
                        />
                      </>
                    )}
                    <MetricItem label="状态" value={item.is_available ? "可用" : "未知"} />
                  </div>

                  <div className="model-pricing-panel">
                    <div className="model-pricing-panel-title">基础价格源</div>
                    <div className="model-pricing-grid">
                      {currentIsFixed ? (
                        <MetricItem
                          label="内置固定价"
                          value={formatPrice(item.base_fixed_price, item.base_pricing_currency, item.base_pricing_unit)}
                        />
                      ) : (
                        <>
                          <MetricItem
                            label="内置输入价"
                            value={formatPrice(item.base_input_price_per_1m, item.base_pricing_currency, item.base_pricing_unit)}
                          />
                          <MetricItem
                            label="内置输出价"
                            value={formatPrice(item.base_output_price_per_1m, item.base_pricing_currency, item.base_pricing_unit)}
                          />
                          <MetricItem
                            label="内置请求费"
                            value={formatPrice(item.base_request_price, item.base_pricing_currency, "request")}
                          />
                        </>
                      )}
                      <MetricItem label="计费币种" value={formatPricingCurrency(item.pricing_currency)} />
                      <MetricItem label="计费单位" value={formatPricingUnit(item.pricing_unit)} />
                      <MetricItem label="计费形态" value={formatBillingShape(item)} />
                      <MetricItem label="当前来源" value={formatPricingSource(item)} />
                      <MetricItem label="最后更新时间" value={formatPricingUpdatedAt(item.pricing_updated_at)} />
                    </div>
                    {item.pricing_note ? (
                      <div className="model-pricing-note">备注：{item.pricing_note}</div>
                    ) : null}
                  </div>

                  {isEditing ? (
                    <div className="model-editor">
                      <div className="model-editor-title">编辑基础价格</div>
                      <div className="model-editor-grid">
                        <label className="form-row">
                          <span className="form-label">计费币种</span>
                          <select
                            className="form-input"
                            value={draft.currency}
                            onChange={(e) => updateDraft(key, { currency: e.target.value })}
                          >
                            <option value="USD">USD</option>
                            <option value="CNY">CNY</option>
                          </select>
                        </label>
                        <label className="form-row">
                          <span className="form-label">计费单位</span>
                          <select
                            className="form-input"
                            value={draft.unit}
                            onChange={(e) => updateDraft(key, { unit: e.target.value })}
                          >
                            <option value="1m_tokens">1M token</option>
                            <option value="request">每次请求</option>
                            <option value="image">每张图像</option>
                          </select>
                        </label>
                      </div>
                      <div className="model-editor-grid">
                        {draftIsFixed ? (
                          <label className="form-row">
                            <span className="form-label">
                              固定价 / {formatPricingUnit(draft.unit)} {draft.currency ? `(${draft.currency})` : ""}
                            </span>
                            <input
                              className="form-input"
                              value={draft.fixed}
                              onChange={(e) => updateDraft(key, { fixed: e.target.value })}
                              placeholder="例如 0.02"
                            />
                          </label>
                        ) : (
                          <>
                            <label className="form-row">
                              <span className="form-label">
                                输入价 / {formatPricingUnit(draft.unit)} {draft.currency ? `(${draft.currency})` : ""}
                              </span>
                              <input
                                className="form-input"
                                value={draft.input}
                                onChange={(e) => updateDraft(key, { input: e.target.value })}
                                placeholder="例如 2.5"
                              />
                            </label>
                            <label className="form-row">
                              <span className="form-label">
                                输出价 / {formatPricingUnit(draft.unit)} {draft.currency ? `(${draft.currency})` : ""}
                              </span>
                              <input
                                className="form-input"
                                value={draft.output}
                                onChange={(e) => updateDraft(key, { output: e.target.value })}
                                placeholder="例如 10"
                              />
                            </label>
                            <label className="form-row">
                              <span className="form-label">
                                请求费 / 每次请求 {draft.currency ? `(${draft.currency})` : ""}
                              </span>
                              <input
                                className="form-input"
                                value={draft.request}
                                onChange={(e) => updateDraft(key, { request: e.target.value })}
                                placeholder="例如 0.006"
                              />
                            </label>
                          </>
                        )}
                      </div>
                      <label className="form-row">
                        <span className="form-label">备注</span>
                        <textarea
                          className="form-input model-editor-textarea"
                          value={draft.note}
                          onChange={(e) => updateDraft(key, { note: e.target.value })}
                          placeholder="记录价格口径、适用说明或手工覆盖原因"
                        />
                      </label>
                      <div className="model-editor-actions">
                        <button className="btn btn-primary" onClick={() => void handleSave(item)} disabled={isSaving}>
                          {isSaving ? "保存中..." : "保存"}
                        </button>
                        <button className="btn btn-secondary" onClick={cancelEdit} disabled={isSaving}>
                          取消
                        </button>
                        <button
                          className="btn btn-danger"
                          onClick={() => void handleReset(item)}
                          disabled={isSaving || item.pricing_source !== "manual"}
                        >
                          恢复内置
                        </button>
                      </div>
                    </div>
                  ) : null}
                </article>
              );
            })}
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
