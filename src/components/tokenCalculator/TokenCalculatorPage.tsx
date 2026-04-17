import {
  useMemo,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import {
  api,
  type FetchRemoteModelPricingResponse,
  type TokenCalculatorRemoteModelPricing,
} from "../../lib/api";
import "./TokenCalculatorPage.css";

type CalculatorMode = "remote" | "compare" | "manual";
type RemoteGroupMode = "recommended" | string;
type CompareMetric = "input" | "output" | "cache" | "fixed";
type PriceKind = "per_1m" | "per_request";

interface ManualModelRow {
  id: string;
  model: string;
  input: string;
  output: string;
  cacheRead: string;
}

interface SitePricingState {
  baseUrl: string;
  apiKey: string;
  localAmount: string;
  usdAmount: string;
  loading: boolean;
  error: string;
  result: FetchRemoteModelPricingResponse | null;
  selectedGroup: RemoteGroupMode;
}

interface PriceTableRow extends TokenCalculatorRemoteModelPricing {
  key: string;
  display_name: string;
  billing_mode: "token" | "fixed" | "unknown";
  active_group?: string | null;
  active_group_ratio?: number | null;
  fixed_price_local?: number | null;
  pricing_note?: string | null;
}

interface CompareRow {
  id: string;
  display_name: string;
  description?: string | null;
  left: PriceTableRow;
  right: PriceTableRow;
  deltaUsd?: number | null;
  deltaRmb?: number | null;
  cheaperSide: "left" | "right" | "same" | "unknown";
}

function createManualRow(): ManualModelRow {
  return {
    id: `manual-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    model: "",
    input: "",
    output: "",
    cacheRead: "",
  };
}

function createSiteState(baseUrl = "https://api.opusclaw.me"): SitePricingState {
  return {
    baseUrl,
    apiKey: "",
    localAmount: "10",
    usdAmount: "100",
    loading: false,
    error: "",
    result: null,
    selectedGroup: "recommended",
  };
}

function parseNumber(value: string): number | undefined {
  const normalized = value.trim();
  if (!normalized) return undefined;
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function computeRatio(localAmount: string, usdAmount: string): number | undefined {
  const local = parseNumber(localAmount);
  const usd = parseNumber(usdAmount);
  if (local == null || usd == null || usd <= 0) return undefined;
  return local / usd;
}

function formatUsdPer1M(value?: number | null) {
  return value == null ? "—" : `$${value.toFixed(4)} / 1M`;
}

function formatRmbPer1M(value?: number | null, ratio?: number) {
  if (value == null || ratio == null || !Number.isFinite(ratio)) return "—";
  return `¥${(value * ratio).toFixed(4)} / 1M`;
}

function formatUsdPerRequest(value?: number | null) {
  return value == null ? "—" : `$${value.toFixed(4)} / 次`;
}

function formatRmbPerRequest(value?: number | null, ratio?: number) {
  if (value == null || ratio == null || !Number.isFinite(ratio)) return "—";
  return `¥${(value * ratio).toFixed(4)} / 次`;
}

function formatSignedUsd(value?: number | null, kind: PriceKind = "per_1m") {
  if (value == null || !Number.isFinite(value)) return "—";
  const sign = value > 0 ? "+" : value < 0 ? "-" : "±";
  const abs = Math.abs(value);
  return kind === "per_request"
    ? `${sign}$${abs.toFixed(4)} / 次`
    : `${sign}$${abs.toFixed(4)} / 1M`;
}

function formatSignedRmb(value?: number | null, kind: PriceKind = "per_1m") {
  if (value == null || !Number.isFinite(value)) return "—";
  const sign = value > 0 ? "+" : value < 0 ? "-" : "±";
  const abs = Math.abs(value);
  return kind === "per_request"
    ? `${sign}¥${abs.toFixed(4)} / 次`
    : `${sign}¥${abs.toFixed(4)} / 1M`;
}

function getSiteDisplayName(baseUrl: string, fallback: string) {
  try {
    return new URL(baseUrl.trim()).host || fallback;
  } catch {
    return baseUrl.trim() || fallback;
  }
}

function getCompareMetricLabel(metric: CompareMetric) {
  switch (metric) {
    case "input":
      return "输入价";
    case "output":
      return "补全价";
    case "cache":
      return "缓存读";
    case "fixed":
      return "固定价";
    default:
      return "价格";
  }
}

function resolveNewApiGroup(
  item: TokenCalculatorRemoteModelPricing,
  result: FetchRemoteModelPricingResponse,
  selectedGroup: RemoteGroupMode,
) {
  if (selectedGroup !== "recommended") {
    const enabled = item.enable_groups || [];
    const exists = Object.prototype.hasOwnProperty.call(result.group_ratios, selectedGroup);
    const allowed = enabled.length === 0 || enabled.includes(selectedGroup);
    if (exists && allowed) {
      return {
        group: selectedGroup,
        ratio: result.group_ratios[selectedGroup],
      };
    }

    return {
      group: selectedGroup,
      ratio: undefined,
      invalid: true,
    };
  }

  const candidate =
    item.recommended_group ||
    (item.enable_groups || []).find((group) =>
      Object.prototype.hasOwnProperty.call(result.group_ratios, group)
    ) ||
    Object.keys(result.group_ratios)[0];

  return {
    group: candidate || null,
    ratio: candidate ? result.group_ratios[candidate] : undefined,
    invalid: false,
  };
}

function buildRemoteRows(
  result: FetchRemoteModelPricingResponse | null,
  selectedGroup: RemoteGroupMode,
  ratio: number | undefined,
  prefix: string,
): PriceTableRow[] {
  return (result?.models || []).map((item) => {
    const display_name = item.name?.trim() || item.id;

    if (result?.provider_kind !== "newapi") {
      return {
        ...item,
        key: `${prefix}-${item.id}`,
        display_name,
        billing_mode: item.fixed_price_usd != null ? "fixed" : "token",
        active_group: null,
        active_group_ratio: null,
        fixed_price_local:
          ratio != null && item.fixed_price_usd != null ? item.fixed_price_usd * ratio : null,
        pricing_note: null,
      };
    }

    const groupInfo = resolveNewApiGroup(item, result, selectedGroup);
    const quotaPerUnit = result.quota_per_unit || 500000;
    const usdPer1mFactor = 1_000_000 / quotaPerUnit;

    if (groupInfo.invalid) {
      return {
        ...item,
        key: `${prefix}-${item.id}`,
        display_name,
        billing_mode: item.quota_type === 1 ? "fixed" : "unknown",
        active_group: groupInfo.group,
        active_group_ratio: null,
        input_price_per_1m: undefined,
        output_price_per_1m: undefined,
        cache_read_price_per_1m: undefined,
        fixed_price_usd: undefined,
        fixed_price_local: null,
        pricing_note: "该模型不支持当前分组",
      };
    }

    if (item.quota_type === 1) {
      const fixedPriceUsd =
        item.model_price != null && groupInfo.ratio != null
          ? item.model_price * groupInfo.ratio
          : item.fixed_price_usd;
      return {
        ...item,
        key: `${prefix}-${item.id}`,
        display_name,
        billing_mode: "fixed",
        active_group: groupInfo.group,
        active_group_ratio: groupInfo.ratio ?? null,
        fixed_price_usd: fixedPriceUsd,
        fixed_price_local:
          ratio != null && fixedPriceUsd != null ? fixedPriceUsd * ratio : null,
        pricing_note: "按次计费模型",
      };
    }

    const inputPrice =
      item.model_ratio != null && groupInfo.ratio != null
        ? item.model_ratio * groupInfo.ratio * usdPer1mFactor
        : item.input_price_per_1m;
    const outputPrice =
      inputPrice != null && item.completion_ratio != null
        ? inputPrice * item.completion_ratio
        : item.output_price_per_1m;
    const cacheReadPrice =
      inputPrice != null && item.cache_ratio != null
        ? inputPrice * item.cache_ratio
        : item.cache_read_price_per_1m;

    return {
      ...item,
      key: `${prefix}-${item.id}`,
      display_name,
      billing_mode: "token",
      active_group: groupInfo.group,
      active_group_ratio: groupInfo.ratio ?? null,
      input_price_per_1m: inputPrice,
      output_price_per_1m: outputPrice,
      cache_read_price_per_1m: cacheReadPrice,
      fixed_price_local: null,
      pricing_note:
        groupInfo.group && groupInfo.ratio != null
          ? `NewAPI: model_ratio × group_ratio × ${usdPer1mFactor.toFixed(0)}`
          : null,
    };
  });
}

function buildRemoteGroupOptions(result: FetchRemoteModelPricingResponse | null) {
  if (!result || result.provider_kind !== "newapi") return [];
  return [
    { value: "recommended", label: "按模型推荐分组" },
    ...Object.entries(result.group_ratios).map(([group, value]) => ({
      value: group,
      label: `${result.group_labels[group] || group} (${group} × ${value})`,
    })),
  ];
}

function getMetricRmb(row: PriceTableRow, metric: CompareMetric, ratio: number | undefined) {
  switch (metric) {
    case "fixed":
      return row.fixed_price_usd != null && ratio != null ? row.fixed_price_usd * ratio : null;
    case "output":
      return row.output_price_per_1m != null && ratio != null ? row.output_price_per_1m * ratio : null;
    case "cache":
      return row.cache_read_price_per_1m != null && ratio != null ? row.cache_read_price_per_1m * ratio : null;
    case "input":
    default:
      return row.input_price_per_1m != null && ratio != null ? row.input_price_per_1m * ratio : null;
  }
}

function formatMetricUsd(row: PriceTableRow, metric: CompareMetric) {
  switch (metric) {
    case "fixed":
      return formatUsdPerRequest(row.fixed_price_usd);
    case "output":
      return formatUsdPer1M(row.output_price_per_1m);
    case "cache":
      return formatUsdPer1M(row.cache_read_price_per_1m);
    case "input":
    default:
      return formatUsdPer1M(row.input_price_per_1m);
  }
}

function formatMetricRmb(row: PriceTableRow, metric: CompareMetric, ratio: number | undefined) {
  switch (metric) {
    case "fixed":
      return formatRmbPerRequest(row.fixed_price_usd, ratio);
    case "output":
      return formatRmbPer1M(row.output_price_per_1m, ratio);
    case "cache":
      return formatRmbPer1M(row.cache_read_price_per_1m, ratio);
    case "input":
    default:
      return formatRmbPer1M(row.input_price_per_1m, ratio);
  }
}

function compareMetricKind(metric: CompareMetric): PriceKind {
  return metric === "fixed" ? "per_request" : "per_1m";
}

function buildCompareRows(
  leftRows: PriceTableRow[],
  rightRows: PriceTableRow[],
  leftRatio: number | undefined,
  rightRatio: number | undefined,
  metric: CompareMetric,
): CompareRow[] {
  const leftMap = new Map(leftRows.map((row) => [row.id, row]));
  const rightMap = new Map(rightRows.map((row) => [row.id, row]));

  const rows: CompareRow[] = [];
  for (const [id, left] of leftMap.entries()) {
    const right = rightMap.get(id);
    if (!right) continue;

    const leftUsd = metric === "fixed" ? left.fixed_price_usd : (
      metric === "output" ? left.output_price_per_1m : metric === "cache" ? left.cache_read_price_per_1m : left.input_price_per_1m
    );
    const rightUsd = metric === "fixed" ? right.fixed_price_usd : (
      metric === "output" ? right.output_price_per_1m : metric === "cache" ? right.cache_read_price_per_1m : right.input_price_per_1m
    );
    const leftRmb = getMetricRmb(left, metric, leftRatio);
    const rightRmb = getMetricRmb(right, metric, rightRatio);

    if (leftUsd == null && rightUsd == null && leftRmb == null && rightRmb == null) {
      continue;
    }

    let cheaperSide: CompareRow["cheaperSide"] = "unknown";
    let deltaUsd: number | null = null;
    let deltaRmb: number | null = null;

    if (leftRmb != null && rightRmb != null) {
      const diff = leftRmb - rightRmb;
      deltaRmb = diff;
      deltaUsd = leftUsd != null && rightUsd != null ? leftUsd - rightUsd : null;
      cheaperSide = Math.abs(diff) < 0.0000001 ? "same" : diff < 0 ? "left" : "right";
    } else if (leftUsd != null && rightUsd != null) {
      const diff = leftUsd - rightUsd;
      deltaUsd = diff;
      cheaperSide = Math.abs(diff) < 0.0000001 ? "same" : diff < 0 ? "left" : "right";
    }

    rows.push({
      id,
      display_name: left.display_name || right.display_name || id,
      description: left.description || right.description || null,
      left,
      right,
      deltaUsd,
      deltaRmb,
      cheaperSide,
    });
  }

  return rows.sort((a, b) => a.display_name.localeCompare(b.display_name));
}

export default function TokenCalculatorPage() {
  const [mode, setMode] = useState<CalculatorMode>("remote");
  const [search, setSearch] = useState("");
  const [compareMetric, setCompareMetric] = useState<CompareMetric>("input");
  const [singleSite, setSingleSite] = useState<SitePricingState>(createSiteState());
  const [leftSite, setLeftSite] = useState<SitePricingState>(createSiteState("https://api.opusclaw.me"));
  const [rightSite, setRightSite] = useState<SitePricingState>(createSiteState("https://api.opusclaw.me"));
  const [manualRows, setManualRows] = useState<ManualModelRow[]>([
    {
      id: "manual-example",
      model: "claude-opus-4-6",
      input: "12.5",
      output: "62.5",
      cacheRead: "1.25",
    },
  ]);

  const singleRatio = useMemo(
    () => computeRatio(singleSite.localAmount, singleSite.usdAmount),
    [singleSite.localAmount, singleSite.usdAmount]
  );
  const leftRatio = useMemo(
    () => computeRatio(leftSite.localAmount, leftSite.usdAmount),
    [leftSite.localAmount, leftSite.usdAmount]
  );
  const rightRatio = useMemo(
    () => computeRatio(rightSite.localAmount, rightSite.usdAmount),
    [rightSite.localAmount, rightSite.usdAmount]
  );

  const singleRows = useMemo(
    () => buildRemoteRows(singleSite.result, singleSite.selectedGroup, singleRatio, "single"),
    [singleSite.result, singleSite.selectedGroup, singleRatio]
  );
  const leftRows = useMemo(
    () => buildRemoteRows(leftSite.result, leftSite.selectedGroup, leftRatio, "left"),
    [leftSite.result, leftSite.selectedGroup, leftRatio]
  );
  const rightRows = useMemo(
    () => buildRemoteRows(rightSite.result, rightSite.selectedGroup, rightRatio, "right"),
    [rightSite.result, rightSite.selectedGroup, rightRatio]
  );

  const manualComputedRows = useMemo<PriceTableRow[]>(() => {
    return manualRows.reduce<PriceTableRow[]>((rows, row) => {
      const hasAnyValue =
        row.model.trim() ||
        row.input.trim() ||
        row.output.trim() ||
        row.cacheRead.trim();
      if (!hasAnyValue) return rows;

      rows.push({
        key: row.id,
        id: row.model.trim() || `未命名模型-${row.id}`,
        name: row.model.trim() || null,
        display_name: row.model.trim() || "未命名模型",
        description: null,
        input_price_per_1m: parseNumber(row.input),
        output_price_per_1m: parseNumber(row.output),
        cache_read_price_per_1m: parseNumber(row.cacheRead),
        billing_mode: "token",
        active_group: null,
        active_group_ratio: null,
        fixed_price_local: null,
        pricing_note: null,
      });

      return rows;
    }, []);
  }, [manualRows]);

  const compareRows = useMemo(
    () => buildCompareRows(leftRows, rightRows, leftRatio, rightRatio, compareMetric),
    [leftRows, rightRows, leftRatio, rightRatio, compareMetric]
  );

  const filteredSingleRows = useMemo(() => {
    const query = search.trim().toLowerCase();
    return (mode === "manual" ? manualComputedRows : singleRows).filter((item) => {
      if (!query) return true;
      return (
        item.display_name.toLowerCase().includes(query) ||
        item.id.toLowerCase().includes(query)
      );
    });
  }, [search, singleRows, manualComputedRows, mode]);

  const filteredCompareRows = useMemo(() => {
    const query = search.trim().toLowerCase();
    return compareRows.filter((item) => {
      if (!query) return true;
      return (
        item.display_name.toLowerCase().includes(query) ||
        item.id.toLowerCase().includes(query)
      );
    });
  }, [search, compareRows]);

  const singlePricedCount = useMemo(() => {
    const rows = mode === "manual" ? manualComputedRows : singleRows;
    return rows.filter(
      (item) =>
        item.input_price_per_1m != null ||
        item.output_price_per_1m != null ||
        item.cache_read_price_per_1m != null ||
        item.fixed_price_usd != null
    ).length;
  }, [mode, manualComputedRows, singleRows]);

  const compareCheaperStats = useMemo(() => {
    return filteredCompareRows.reduce(
      (acc, row) => {
        if (row.cheaperSide === "left") acc.left += 1;
        if (row.cheaperSide === "right") acc.right += 1;
        if (row.cheaperSide === "same") acc.same += 1;
        return acc;
      },
      { left: 0, right: 0, same: 0 }
    );
  }, [filteredCompareRows]);

  const singleGroupOptions = useMemo(
    () => buildRemoteGroupOptions(singleSite.result),
    [singleSite.result]
  );
  const leftGroupOptions = useMemo(
    () => buildRemoteGroupOptions(leftSite.result),
    [leftSite.result]
  );
  const rightGroupOptions = useMemo(
    () => buildRemoteGroupOptions(rightSite.result),
    [rightSite.result]
  );

  const fetchSitePricing = async (
    site: SitePricingState,
    setSite: Dispatch<SetStateAction<SitePricingState>>
  ) => {
    setSite((prev) => ({ ...prev, loading: true, error: "" }));
    try {
      const result = await api.tokenCalculator.fetchRemotePricing({
        base_url: site.baseUrl,
        api_key: site.apiKey.trim() || undefined,
      });
      setSite((prev) => ({
        ...prev,
        loading: false,
        error: "",
        result,
        selectedGroup: "recommended",
      }));
    } catch (error) {
      setSite((prev) => ({
        ...prev,
        loading: false,
        result: null,
        error: String(error),
      }));
    }
  };

  const updateSite = (
    setSite: Dispatch<SetStateAction<SitePricingState>>,
    patch: Partial<SitePricingState>
  ) => {
    setSite((prev) => ({ ...prev, ...patch }));
  };

  const updateManualRow = (rowId: string, patch: Partial<ManualModelRow>) => {
    setManualRows((prev) =>
      prev.map((row) => (row.id === rowId ? { ...row, ...patch } : row))
    );
  };

  const addManualRow = () => {
    setManualRows((prev) => [...prev, createManualRow()]);
  };

  const removeManualRow = (rowId: string) => {
    setManualRows((prev) => {
      if (prev.length <= 1) {
        return [{ ...createManualRow(), id: prev[0]?.id || "manual-empty" }];
      }
      return prev.filter((row) => row.id !== rowId);
    });
  };

  const leftSiteName = getSiteDisplayName(leftSite.baseUrl, "A 站");
  const rightSiteName = getSiteDisplayName(rightSite.baseUrl, "B 站");

  return (
    <div className="token-calculator-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">TOKEN 计算器</h1>
          <p className="page-subtitle">单站换算、双站比价、手动录入统一放在一个工作台。</p>
        </div>
      </div>

      <div className="token-calculator-body">
        {mode === "compare" ? (
          <div className="token-stats">
            <StatCard label="A 站充值比" value={leftRatio == null ? "请填写" : `¥${leftRatio.toFixed(4)} / $1`} />
            <StatCard label="B 站充值比" value={rightRatio == null ? "请填写" : `¥${rightRatio.toFixed(4)} / $1`} />
            <StatCard label="共同模型数" value={String(compareRows.length)} />
            <StatCard label="当前对比项" value={getCompareMetricLabel(compareMetric)} />
          </div>
        ) : (
          <div className="token-stats">
            <StatCard
              label="充值比例"
              value={`¥${singleSite.localAmount || "0"} : $${singleSite.usdAmount || "0"}`}
            />
            <StatCard
              label="美元成本"
              value={singleRatio == null ? "请填写" : `¥${singleRatio.toFixed(4)} / $1`}
            />
            <StatCard
              label="当前模型数"
              value={String(mode === "manual" ? manualComputedRows.length : singleRows.length)}
            />
            <StatCard label="已识别定价" value={String(singlePricedCount)} />
          </div>
        )}

        <div className="token-layout">
          <section className="card token-panel">
            <div className="token-panel-header">
              <div>
                <h3>计算模式</h3>
                <p>单站换算、双站对比、手动价格录入。</p>
              </div>
              <div className="token-mode-switch">
                <button
                  className={`token-mode-btn ${mode === "remote" ? "active" : ""}`}
                  onClick={() => setMode("remote")}
                >
                  单站换算
                </button>
                <button
                  className={`token-mode-btn ${mode === "compare" ? "active" : ""}`}
                  onClick={() => setMode("compare")}
                >
                  双站对比
                </button>
                <button
                  className={`token-mode-btn ${mode === "manual" ? "active" : ""}`}
                  onClick={() => setMode("manual")}
                >
                  手动填写
                </button>
              </div>
            </div>

            {mode === "remote" && (
              <div className="token-remote-block">
                <div className="token-form-grid">
                  <label className="form-row">
                    <span className="form-label">充值人民币</span>
                    <input
                      className="form-input"
                      value={singleSite.localAmount}
                      onChange={(e) => updateSite(setSingleSite, { localAmount: e.target.value })}
                      placeholder="例如 10"
                    />
                  </label>
                  <label className="form-row">
                    <span className="form-label">到账美元</span>
                    <input
                      className="form-input"
                      value={singleSite.usdAmount}
                      onChange={(e) => updateSite(setSingleSite, { usdAmount: e.target.value })}
                      placeholder="例如 100"
                    />
                  </label>
                </div>
                <div className="form-hint">
                  公式：1M RMB 价格 = 1M USD 价格 × (充值人民币 / 到账美元)
                </div>
                <label className="form-row">
                  <span className="form-label">中转 API 地址</span>
                  <input
                    className="form-input"
                    value={singleSite.baseUrl}
                    onChange={(e) => updateSite(setSingleSite, { baseUrl: e.target.value })}
                    placeholder="https://api.opusclaw.me"
                  />
                </label>
                <label className="form-row">
                  <span className="form-label">API Key（可选）</span>
                  <input
                    className="form-input"
                    value={singleSite.apiKey}
                    onChange={(e) => updateSite(setSingleSite, { apiKey: e.target.value })}
                    placeholder="如果该中转站拉取模型需要鉴权，可填写"
                  />
                </label>
                <div className="token-action-row">
                  <button
                    className="btn btn-primary"
                    onClick={() => void fetchSitePricing(singleSite, setSingleSite)}
                    disabled={singleSite.loading}
                  >
                    {singleSite.loading ? "拉取中..." : "拉取模型与价格"}
                  </button>
                  {singleSite.result && (
                    <span className="token-source-text">
                      来源接口：{singleSite.result.source_endpoint}
                    </span>
                  )}
                </div>
                {singleSite.result?.provider_kind === "newapi" && (
                  <div className="token-newapi-box">
                    <div className="token-newapi-head">
                      <strong>已识别为 NewAPI</strong>
                      <span>先按分组倍率算出 USD / 1M，再换算为 RMB / 1M</span>
                    </div>
                    <div className="token-form-grid">
                      <label className="form-row">
                        <span className="form-label">计费分组</span>
                        <select
                          className="form-input"
                          value={singleSite.selectedGroup}
                          onChange={(e) =>
                            updateSite(setSingleSite, { selectedGroup: e.target.value })
                          }
                        >
                          {singleGroupOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </label>
                      <div className="token-newapi-summary">
                        <span>Quota 基准</span>
                        <strong>
                          {singleSite.result.quota_per_unit
                            ? `1 USD = ${singleSite.result.quota_per_unit.toLocaleString()} quota`
                            : "默认 500,000 quota"}
                        </strong>
                      </div>
                    </div>
                  </div>
                )}
                {singleSite.error && <div className="token-error-box">{singleSite.error}</div>}
                {singleSite.result?.warnings?.length ? (
                  <div className="token-warning-box">
                    {singleSite.result.warnings.map((warning, index) => (
                      <div key={`${warning}-${index}`}>{warning}</div>
                    ))}
                  </div>
                ) : null}
              </div>
            )}

            {mode === "compare" && (
              <div className="token-compare-block">
                <div className="token-compare-toolbar">
                  <label className="form-row token-compare-metric">
                    <span className="form-label">对比项</span>
                    <select
                      className="form-input"
                      value={compareMetric}
                      onChange={(e) => setCompareMetric(e.target.value as CompareMetric)}
                    >
                      <option value="input">输入价</option>
                      <option value="output">补全价</option>
                      <option value="cache">缓存读</option>
                      <option value="fixed">固定价</option>
                    </select>
                  </label>
                  <div className="token-inline-actions">
                    <button
                      className="btn btn-primary"
                      onClick={() => {
                        void fetchSitePricing(leftSite, setLeftSite);
                        void fetchSitePricing(rightSite, setRightSite);
                      }}
                      disabled={leftSite.loading || rightSite.loading}
                    >
                      {leftSite.loading || rightSite.loading ? "对比中..." : "同时拉取两站"}
                    </button>
                  </div>
                </div>

                <div className="token-compare-sites">
                  <CompareSiteCard
                    title="A 站"
                    site={leftSite}
                    setSite={setLeftSite}
                    groupOptions={leftGroupOptions}
                    onFetch={() => void fetchSitePricing(leftSite, setLeftSite)}
                  />
                  <CompareSiteCard
                    title="B 站"
                    site={rightSite}
                    setSite={setRightSite}
                    groupOptions={rightGroupOptions}
                    onFetch={() => void fetchSitePricing(rightSite, setRightSite)}
                  />
                </div>
              </div>
            )}

            {mode === "manual" && (
              <div className="token-manual-block">
                <div className="token-form-grid">
                  <label className="form-row">
                    <span className="form-label">充值人民币</span>
                    <input
                      className="form-input"
                      value={singleSite.localAmount}
                      onChange={(e) => updateSite(setSingleSite, { localAmount: e.target.value })}
                      placeholder="例如 10"
                    />
                  </label>
                  <label className="form-row">
                    <span className="form-label">到账美元</span>
                    <input
                      className="form-input"
                      value={singleSite.usdAmount}
                      onChange={(e) => updateSite(setSingleSite, { usdAmount: e.target.value })}
                      placeholder="例如 100"
                    />
                  </label>
                </div>
                <div className="token-manual-head">
                  <div>
                    <h3>手动模型定价</h3>
                    <p>按 USD / 1M 录入，结果区自动换算 RMB / 1M。</p>
                  </div>
                  <button className="btn btn-ghost btn-sm" onClick={addManualRow}>
                    新增模型
                  </button>
                </div>

                <div className="token-manual-list">
                  {manualRows.map((row) => (
                    <div key={row.id} className="token-manual-row">
                      <input
                        className="form-input"
                        value={row.model}
                        onChange={(e) => updateManualRow(row.id, { model: e.target.value })}
                        placeholder="模型名，例如 claude-opus-4-6"
                      />
                      <input
                        className="form-input"
                        value={row.input}
                        onChange={(e) => updateManualRow(row.id, { input: e.target.value })}
                        placeholder="输入价格"
                      />
                      <input
                        className="form-input"
                        value={row.output}
                        onChange={(e) => updateManualRow(row.id, { output: e.target.value })}
                        placeholder="补全价格"
                      />
                      <input
                        className="form-input"
                        value={row.cacheRead}
                        onChange={(e) => updateManualRow(row.id, { cacheRead: e.target.value })}
                        placeholder="缓存读取价格"
                      />
                      <button
                        className="btn btn-danger btn-sm"
                        onClick={() => removeManualRow(row.id)}
                      >
                        删除
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </section>

          <section className="card token-panel token-highlight-panel">
            <div className="token-highlight-badge">
              {mode === "compare" ? "Compare" : "Calculator"}
            </div>
            <h3>{mode === "compare" ? "对比摘要" : "折算摘要"}</h3>
            <p className="token-highlight-text">
              {mode === "compare"
                ? "A/B 两边独立计算充值比与分组倍率，只展示共同模型，并直接看哪边更便宜。"
                : "结果区统一展示上游价格与按充值比换算后的人民币价格，适合做中转、发卡和内部报价基线。"}
            </p>

            {mode === "compare" ? (
              <>
                <div className="token-formula-card">
                  <div className="token-formula-label">当前对比项</div>
                  <div className="token-formula-value">{getCompareMetricLabel(compareMetric)}</div>
                </div>
                <div className="token-highlight-grid">
                  <MetricCard label="A 站更便宜" value={`${compareCheaperStats.left} 个`} />
                  <MetricCard label="B 站更便宜" value={`${compareCheaperStats.right} 个`} />
                  <MetricCard label="价格持平" value={`${compareCheaperStats.same} 个`} />
                </div>
              </>
            ) : (
              <>
                <div className="token-formula-card">
                  <div className="token-formula-label">当前充值比</div>
                  <div className="token-formula-value">
                    {singleRatio == null ? "请先填写充值比" : `1 美元成本 = ¥${singleRatio.toFixed(4)}`}
                  </div>
                </div>
                <div className="token-highlight-grid">
                  <MetricCard
                    label="输入价 1M RMB 示例"
                    value={singleRatio == null ? "—" : formatRmbPer1M(12.5, singleRatio)}
                  />
                  <MetricCard
                    label="补全价 1M RMB 示例"
                    value={singleRatio == null ? "—" : formatRmbPer1M(62.5, singleRatio)}
                  />
                  <MetricCard
                    label="缓存读 1M RMB 示例"
                    value={singleRatio == null ? "—" : formatRmbPer1M(1.25, singleRatio)}
                  />
                </div>
              </>
            )}

            <div className="token-compact-notes">
              <span>支持 NewAPI 分组倍率</span>
              <span>支持按次计费模型</span>
              <span>支持双站共同模型对比</span>
            </div>
          </section>
        </div>

        <section className="card token-results-panel">
          <div className="token-results-header">
            <div>
              <h3>{mode === "compare" ? "站点价格对比" : "模型价格结果"}</h3>
              <p>
                {mode === "compare"
                  ? "共同模型里对比 A/B 两边的单项价格，同一格上方是 USD，下方是 RMB。"
                  : "同一单元格上方是 USD，下方是 RMB。"}
              </p>
            </div>
            <input
              className="form-input token-search-input"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索模型名"
            />
          </div>

          {mode === "compare" ? (
            filteredCompareRows.length === 0 ? (
              <div className="empty-state token-empty-state">
                <div className="empty-state-icon">🧾</div>
                <h3 style={{ color: "var(--color-text-secondary)" }}>先拉取两边站点的模型价格</h3>
                <p>结果区只显示共同模型，并按当前对比项输出价差和更便宜的一边。</p>
              </div>
            ) : (
              <div className="token-table-wrap">
                <table className="token-price-table token-compare-table">
                  <thead>
                    <tr>
                      <th>模型</th>
                      <th>{leftSiteName}</th>
                      <th>{rightSiteName}</th>
                      <th>差价</th>
                      <th>更便宜</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredCompareRows.map((row) => (
                      <tr key={`compare-${row.id}`}>
                        <td>
                          <div
                            className="token-model-cell"
                            title={[row.display_name, row.id, row.description].filter(Boolean).join("\n")}
                          >
                            <strong>{row.display_name}</strong>
                            <span>{row.id}</span>
                            {row.description && row.description.trim() !== row.display_name ? (
                              <em>{row.description}</em>
                            ) : null}
                          </div>
                        </td>
                        <td>
                          <PriceStackCell
                            usd={formatMetricUsd(row.left, compareMetric)}
                            rmb={formatMetricRmb(row.left, compareMetric, leftRatio)}
                            note={row.left.active_group ? `${row.left.active_group} × ${row.left.active_group_ratio ?? "—"}` : row.left.billing_mode === "fixed" ? "按次计费" : "按 Token"}
                          />
                        </td>
                        <td>
                          <PriceStackCell
                            usd={formatMetricUsd(row.right, compareMetric)}
                            rmb={formatMetricRmb(row.right, compareMetric, rightRatio)}
                            note={row.right.active_group ? `${row.right.active_group} × ${row.right.active_group_ratio ?? "—"}` : row.right.billing_mode === "fixed" ? "按次计费" : "按 Token"}
                          />
                        </td>
                        <td>
                          <PriceStackCell
                            usd={formatSignedUsd(row.deltaUsd, compareMetricKind(compareMetric))}
                            rmb={formatSignedRmb(row.deltaRmb, compareMetricKind(compareMetric))}
                            note="A 站减 B 站"
                          />
                        </td>
                        <td>
                          <div className={`token-cheaper-badge ${row.cheaperSide}`}>
                            {row.cheaperSide === "left"
                              ? "A 站"
                              : row.cheaperSide === "right"
                                ? "B 站"
                                : row.cheaperSide === "same"
                                  ? "持平"
                                  : "未知"}
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )
          ) : filteredSingleRows.length === 0 ? (
            <div className="empty-state token-empty-state">
              <div className="empty-state-icon">🧾</div>
              <h3 style={{ color: "var(--color-text-secondary)" }}>
                {mode === "remote" ? "先拉取模型与价格" : "先填写至少一个模型"}
              </h3>
              <p>结果会在这里按输入价、补全价、缓存读取价统一换算。</p>
            </div>
          ) : (
            <div className="token-table-wrap">
              <table className="token-price-table">
                <thead>
                  <tr>
                    <th>模型</th>
                    <th>类型</th>
                    <th>计费组</th>
                    <th>固定价</th>
                    <th>输入价</th>
                    <th>补全价</th>
                    <th>缓存读</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredSingleRows.map((row) => (
                    <tr key={row.key}>
                      <td>
                        <div
                          className="token-model-cell"
                          title={[row.display_name, row.id, row.description].filter(Boolean).join("\n")}
                        >
                          <strong>{row.display_name}</strong>
                          <span>{row.id}</span>
                          {row.description && row.description.trim() !== row.display_name ? (
                            <em>{row.description}</em>
                          ) : null}
                        </div>
                      </td>
                      <td>
                        {row.billing_mode === "fixed"
                          ? "按次"
                          : row.billing_mode === "token"
                            ? "按 Token"
                            : "未适配"}
                      </td>
                      <td>
                        {row.active_group ? (
                          <div className="token-group-cell">
                            <strong>{row.active_group}</strong>
                            <span>
                              {row.active_group_ratio != null
                                ? `倍率 × ${row.active_group_ratio}`
                                : row.pricing_note || "—"}
                            </span>
                          </div>
                        ) : (
                          row.pricing_note || "—"
                        )}
                      </td>
                      <td>
                        <PriceStackCell
                          usd={formatUsdPerRequest(row.fixed_price_usd)}
                          rmb={formatRmbPerRequest(row.fixed_price_usd, singleRatio)}
                        />
                      </td>
                      <td>
                        <PriceStackCell
                          usd={formatUsdPer1M(row.input_price_per_1m)}
                          rmb={formatRmbPer1M(row.input_price_per_1m, singleRatio)}
                        />
                      </td>
                      <td>
                        <PriceStackCell
                          usd={formatUsdPer1M(row.output_price_per_1m)}
                          rmb={formatRmbPer1M(row.output_price_per_1m, singleRatio)}
                        />
                      </td>
                      <td>
                        <PriceStackCell
                          usd={formatUsdPer1M(row.cache_read_price_per_1m)}
                          rmb={formatRmbPer1M(row.cache_read_price_per_1m, singleRatio)}
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function CompareSiteCard({
  title,
  site,
  setSite,
  groupOptions,
  onFetch,
}: {
  title: string;
  site: SitePricingState;
  setSite: Dispatch<SetStateAction<SitePricingState>>;
  groupOptions: { value: string; label: string }[];
  onFetch: () => void;
}) {
  return (
    <div className="token-compare-site-card">
      <div className="token-compare-site-head">
        <strong>{title}</strong>
        <span>{getSiteDisplayName(site.baseUrl, title)}</span>
      </div>
      <div className="token-form-grid">
        <label className="form-row">
          <span className="form-label">充值人民币</span>
          <input
            className="form-input"
            value={site.localAmount}
            onChange={(e) => setSite((prev) => ({ ...prev, localAmount: e.target.value }))}
            placeholder="10"
          />
        </label>
        <label className="form-row">
          <span className="form-label">到账美元</span>
          <input
            className="form-input"
            value={site.usdAmount}
            onChange={(e) => setSite((prev) => ({ ...prev, usdAmount: e.target.value }))}
            placeholder="100"
          />
        </label>
      </div>
      <label className="form-row">
        <span className="form-label">中转 API 地址</span>
        <input
          className="form-input"
          value={site.baseUrl}
          onChange={(e) => setSite((prev) => ({ ...prev, baseUrl: e.target.value }))}
          placeholder="https://api.example.com"
        />
      </label>
      <label className="form-row">
        <span className="form-label">API Key（可选）</span>
        <input
          className="form-input"
          value={site.apiKey}
          onChange={(e) => setSite((prev) => ({ ...prev, apiKey: e.target.value }))}
          placeholder="如果需要鉴权可填写"
        />
      </label>
      {site.result?.provider_kind === "newapi" && (
        <label className="form-row">
          <span className="form-label">计费分组</span>
          <select
            className="form-input"
            value={site.selectedGroup}
            onChange={(e) => setSite((prev) => ({ ...prev, selectedGroup: e.target.value }))}
          >
            {groupOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="token-action-row">
        <button className="btn btn-ghost btn-sm" onClick={onFetch} disabled={site.loading}>
          {site.loading ? "拉取中..." : `拉取 ${title}`}
        </button>
        {site.result && <span className="token-source-text">{site.result.source_endpoint}</span>}
      </div>
      {site.error && <div className="token-error-box">{site.error}</div>}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="card token-stat-card">
      <div className="token-stat-value">{value}</div>
      <div className="token-stat-label">{label}</div>
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="token-metric-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function PriceStackCell({
  usd,
  rmb,
  note,
}: {
  usd: string;
  rmb: string;
  note?: string | null;
}) {
  return (
    <div className="token-price-stack">
      <span className="token-price-stack-usd">{usd}</span>
      <span className="token-price-stack-rmb">{rmb}</span>
      {note ? <span className="token-price-stack-note">{note}</span> : null}
    </div>
  );
}
