import type { IdeAccount } from "../../types";

export type AttentionReasonFilter =
  | "expired"
  | "forbidden"
  | "rate_limited"
  | "proxy_disabled"
  | "manually_disabled";

export function formatGeminiQuotaSummary(quotaJson?: string) {
  if (!quotaJson) return "—";
  try {
    const value = JSON.parse(quotaJson);
    const root = value?.quota ?? value;
    const buckets = Array.isArray(root?.buckets) ? root.buckets : [];
    let minPercent: number | null = null;
    for (const bucket of buckets) {
      const fraction = typeof bucket?.remainingFraction === "number"
        ? bucket.remainingFraction
        : typeof bucket?.remainingFraction === "string"
          ? Number(bucket.remainingFraction)
          : NaN;
      if (Number.isFinite(fraction)) {
        const percent = Math.max(0, Math.min(100, Math.round(fraction * 100)));
        minPercent = minPercent == null ? percent : Math.min(minPercent, percent);
      }
    }
    const projectId = typeof value?.project_id === "string" ? value.project_id : null;
    if (minPercent == null) return projectId ? `项目:${projectId}` : "已同步";
    return projectId ? `${minPercent}% · ${projectId}` : `${minPercent}%`;
  } catch {
    return "已同步";
  }
}

export function formatIdePlatformLabel(account: IdeAccount) {
  if (account.origin_platform === "gemini") {
    return account.project_id ? `gemini · ${account.project_id}` : "gemini";
  }
  if (account.origin_platform === "codex") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.account_name || meta.plan_type || meta.auth_mode;
    return extra ? `codex · ${extra}` : "codex";
  }
  if (account.origin_platform === "cursor") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.membership_type || meta.subscription_status;
    return extra ? `cursor · ${extra}` : "cursor";
  }
  if (account.origin_platform === "windsurf") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.plan || meta.user_id;
    return extra ? `windsurf · ${extra}` : "windsurf";
  }
  if (account.origin_platform === "kiro") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `kiro · ${extra}` : "kiro";
  }
  if (account.origin_platform === "qoder") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `qoder · ${extra}` : "qoder";
  }
  if (account.origin_platform === "trae") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `trae · ${extra}` : "trae";
  }
  if (account.origin_platform === "codebuddy") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = account.label || meta.nickname || meta.uid;
    return extra ? `codebuddy · ${extra}` : "codebuddy";
  }
  if (account.origin_platform === "codebuddy_cn") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = account.label || meta.nickname || meta.uid;
    return extra ? `codebuddy_cn · ${extra}` : "codebuddy_cn";
  }
  if (account.origin_platform === "workbuddy") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = account.label || meta.nickname || meta.uid;
    return extra ? `workbuddy · ${extra}` : "workbuddy";
  }
  return account.origin_platform;
}

export function isCodexApiKeyAccount(account: IdeAccount) {
  if (account.origin_platform !== "codex") return false;
  const meta = parseIdeMeta(account.meta_json);
  return meta.auth_mode === "apikey";
}

export function isCurrentIdeAccount(account: IdeAccount, currentMap: Record<string, string | null>) {
  const currentId = currentMap[account.origin_platform];
  return !!currentId && currentId === account.id;
}

export function isIdeNeedsAttention(account: IdeAccount) {
  return account.status !== "active" || account.is_proxy_disabled || !!account.disabled_reason;
}

export function isIdeMatchingAttentionReason(account: IdeAccount, reason: AttentionReasonFilter) {
  switch (reason) {
    case "expired":
      return account.status === "expired";
    case "forbidden":
      return account.status === "forbidden";
    case "rate_limited":
      return account.status === "rate_limited";
    case "proxy_disabled":
      return account.is_proxy_disabled;
    case "manually_disabled":
      return !!account.disabled_reason;
    default:
      return false;
  }
}

export function getAttentionReasonLabel(reason: AttentionReasonFilter) {
  switch (reason) {
    case "expired":
      return "过期";
    case "forbidden":
      return "封禁";
    case "rate_limited":
      return "限流";
    case "proxy_disabled":
      return "代理禁用";
    case "manually_disabled":
      return "人工禁用";
    default:
      return "需关注";
  }
}

export function getAttentionReasonSuggestedTag(reason: AttentionReasonFilter) {
  switch (reason) {
    case "expired":
      return "expired";
    case "forbidden":
      return "forbidden";
    case "rate_limited":
      return "rate-limited";
    case "proxy_disabled":
      return "proxy-disabled";
    case "manually_disabled":
      return "manually-disabled";
    default:
      return "needs-review";
  }
}

export function getCurrentActionLabel(account: IdeAccount) {
  switch (account.origin_platform) {
    case "codex":
      return "设为当前 Codex 账号";
    case "gemini":
      return "设为当前 Gemini 账号";
    case "cursor":
      return "设为当前 Cursor 账号";
    case "windsurf":
      return "设为当前 Windsurf 账号";
    case "kiro":
      return "设为当前 Kiro 账号";
    case "qoder":
      return "设为当前 Qoder 账号";
    case "trae":
      return "设为当前 Trae 账号";
    case "codebuddy":
      return "设为当前 CodeBuddy 账号";
    case "codebuddy_cn":
      return "设为当前 CodeBuddy CN 账号";
    case "workbuddy":
      return "设为当前 WorkBuddy 账号";
    case "zed":
      return "设为当前 Zed 账号";
    default:
      return "设为当前账号";
  }
}

const IDE_REFRESH_TITLE_LABELS: Record<string, string> = {
  codex: "刷新 Codex 配额与资料",
  gemini: "刷新 Gemini 状态与配额",
  cursor: "刷新 Cursor 本地登录态",
  windsurf: "刷新 Windsurf 本地登录态",
  kiro: "刷新 Kiro 本地登录态",
  qoder: "刷新 Qoder 本地登录态",
  trae: "刷新 Trae 本地登录态",
  codebuddy: "刷新 CodeBuddy 本地登录态",
  codebuddy_cn: "刷新 CodeBuddy CN 本地登录态",
  workbuddy: "刷新 WorkBuddy 本地登录态",
  zed: "刷新 Zed 本地登录态",
};

const IDE_REFRESH_SUCCESS_LABELS: Record<string, string> = {
  codex: "Codex 状态与配额已刷新",
  gemini: "Gemini 状态与配额已刷新",
  cursor: "Cursor 本地登录态已刷新",
  windsurf: "Windsurf 本地登录态已刷新",
  kiro: "Kiro 本地登录态已刷新",
  qoder: "Qoder 本地登录态已刷新",
  trae: "Trae 本地登录态已刷新",
  codebuddy: "CodeBuddy 本地登录态已刷新",
  codebuddy_cn: "CodeBuddy CN 本地登录态已刷新",
  workbuddy: "WorkBuddy 本地登录态已刷新",
  zed: "Zed 本地登录态已刷新",
};

export function isIdeRefreshSupported(account: IdeAccount) {
  const platform = account.origin_platform.toLowerCase();
  if (platform === "codex" && isCodexApiKeyAccount(account)) {
    return false;
  }
  return platform in IDE_REFRESH_TITLE_LABELS;
}

export function getIdeRefreshActionLabel(platform: string) {
  const key = platform.toLowerCase();
  return IDE_REFRESH_TITLE_LABELS[key] || `刷新 ${platform} 状态`;
}

export function getIdeRefreshSuccessMessage(platform: string) {
  const key = platform.toLowerCase();
  return IDE_REFRESH_SUCCESS_LABELS[key] || `${platform} 已刷新`;
}

export function getIdeRefreshFailureMessage(platform: string, error: unknown) {
  const key = platform.toLowerCase();
  const prefix = IDE_REFRESH_SUCCESS_LABELS[key]
    ? IDE_REFRESH_SUCCESS_LABELS[key].replace("已刷新", "刷新失败")
    : `${platform} 刷新失败`;
  return `${prefix}: ${error}`;
}

export function formatGeminiQuotaTooltip(quotaJson?: string) {
  if (!quotaJson) return "";
  try {
    const value = JSON.parse(quotaJson);
    const root = value?.quota ?? value;
    const buckets = Array.isArray(root?.buckets) ? root.buckets : [];
    const projectId = typeof value?.project_id === "string" ? value.project_id : "";
    const modelSummaries = buckets
      .map((bucket: any) => {
        const modelId = typeof bucket?.modelId === "string" ? bucket.modelId : "unknown";
        const fraction = typeof bucket?.remainingFraction === "number"
          ? bucket.remainingFraction
          : typeof bucket?.remainingFraction === "string"
            ? Number(bucket.remainingFraction)
            : NaN;
        if (!Number.isFinite(fraction)) return null;
        return `${modelId}: ${Math.max(0, Math.min(100, Math.round(fraction * 100)))}%`;
      })
      .filter(Boolean)
      .slice(0, 4);

    return [projectId ? `Project: ${projectId}` : null, ...modelSummaries]
      .filter(Boolean)
      .join("\n");
  } catch {
    return "";
  }
}

export function formatCursorSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return meta.membership_type || meta.subscription_status || "已同步";
}

export function formatCursorTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.auth_id === "string" ? `Auth ID: ${meta.auth_id}` : null,
    typeof meta.membership_type === "string" ? `Membership: ${meta.membership_type}` : null,
    typeof meta.subscription_status === "string" ? `Subscription: ${meta.subscription_status}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatWindsurfSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return meta.plan || "已同步";
}

export function formatWindsurfTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    typeof meta.plan === "string" ? `Plan: ${meta.plan}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatKiroSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

export function formatKiroTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatQoderSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

export function formatQoderTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatTraeSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

export function formatTraeTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatCodebuddySummary(account: IdeAccount) {
  const quotaSummary = formatCodebuddyQuotaSummary(account.quota_json);
  if (quotaSummary) return quotaSummary;
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.nickname || meta.uid || "已同步";
}

export function formatCodebuddyTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  const quotaTooltip = formatCodebuddyQuotaTooltip(account.quota_json);
  return [
    typeof meta.uid === "string" ? `UID: ${meta.uid}` : null,
    typeof meta.nickname === "string" ? `Nickname: ${meta.nickname}` : null,
    account.label ? `Profile: ${account.label}` : null,
    quotaTooltip,
  ]
    .filter(Boolean)
    .join("\n");
}

export function formatCodexQuotaSummary(quotaJson?: string) {
  if (!quotaJson) return "—";
  try {
    const value = JSON.parse(quotaJson);
    const hourly = typeof value?.hourly_percentage === "number" ? value.hourly_percentage : null;
    const weekly = typeof value?.weekly_percentage === "number" ? value.weekly_percentage : null;
    const planType = typeof value?.plan_type === "string" ? value.plan_type : null;
    if (hourly != null && weekly != null) return `${hourly}% / ${weekly}%`;
    if (hourly != null) return `${hourly}%`;
    if (weekly != null) return `周 ${weekly}%`;
    return planType || "已同步";
  } catch {
    return "已同步";
  }
}

export function formatCodexQuotaTooltip(quotaJson?: string) {
  if (!quotaJson) return "";
  try {
    const value = JSON.parse(quotaJson);
    return [
      typeof value?.plan_type === "string" ? `Plan: ${value.plan_type}` : null,
      typeof value?.hourly_percentage === "number" ? `5h: ${value.hourly_percentage}%` : null,
      typeof value?.weekly_percentage === "number" ? `Weekly: ${value.weekly_percentage}%` : null,
    ]
      .filter(Boolean)
      .join("\n");
  } catch {
    return "";
  }
}

export function formatCodexApiKeyTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  if (meta.auth_mode !== "apikey") return "";
  return [
    "Auth: API Key",
    typeof meta.api_base_url === "string" && meta.api_base_url ? `Base URL: ${meta.api_base_url}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatCodebuddyQuotaSummary(quotaJson?: string) {
  if (!quotaJson) return null;
  try {
    const value = JSON.parse(quotaJson);
    const candidates = [
      pickFiniteNumber(value, ["remaining_credit", "remainingCredit", "credit", "quota_remaining"]),
      pickFiniteNumber(value, ["balance", "remaining", "remaining_amount"]),
      pickFiniteNumber(value, ["userResource", "data", "total", "remaining"]),
      pickFiniteNumber(value, ["userResource", "remaining"]),
      pickFiniteNumber(value, ["dosage", "data", "remaining"]),
    ];
    const firstNumber = candidates.find((item) => item != null);
    if (firstNumber == null) return null;
    return `余额 ${firstNumber}`;
  } catch {
    return null;
  }
}

function formatCodebuddyQuotaTooltip(quotaJson?: string) {
  if (!quotaJson) return null;
  try {
    const value = JSON.parse(quotaJson);
    const lines = [
      describeQuotaField(value, "remaining_credit", "Remaining Credit"),
      describeQuotaField(value, "credit", "Credit"),
      describeQuotaField(value, "quota_remaining", "Quota Remaining"),
      describeQuotaField(value, "balance", "Balance"),
    ].filter(Boolean) as string[];
    if (lines.length === 0) return null;
    return lines.join("\n");
  } catch {
    return null;
  }
}

function describeQuotaField(root: any, key: string, label: string) {
  const value = root?.[key];
  if (value == null) return null;
  if (typeof value === "number") return `${label}: ${value}`;
  if (typeof value === "string" && value.trim()) return `${label}: ${value.trim()}`;
  return null;
}

function pickFiniteNumber(root: any, path: string[]): number | null {
  let cursor = root;
  for (const key of path) {
    if (cursor == null || typeof cursor !== "object") return null;
    cursor = cursor[key];
  }
  if (typeof cursor === "number" && Number.isFinite(cursor)) return cursor;
  if (typeof cursor === "string") {
    const parsed = Number(cursor);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

export function parseIdeMeta(metaJson?: string) {
  if (!metaJson) return {} as Record<string, string>;
  try {
    const value = JSON.parse(metaJson);
    return typeof value === "object" && value ? value as Record<string, string> : {};
  } catch {
    return {} as Record<string, string>;
  }
}
