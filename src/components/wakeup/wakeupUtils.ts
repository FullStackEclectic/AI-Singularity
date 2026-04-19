import type {
  WakeupHistoryItem,
  WakeupTask,
} from "../../lib/api";
import type { IdeAccount } from "../../types";

export const WAKEUP_UNGROUPED_FILTER = "__ungrouped__";

const WAKEUP_CATEGORY_LABELS: Record<string, string> = {
  success: "成功",
  account_not_found: "账号不存在",
  inject_failed: "注入失败",
  timeout: "执行超时",
  command_not_found: "命令不存在",
  permission_denied: "权限不足",
  auth_failed: "鉴权失败",
  rate_limited: "命中限流",
  command_failed: "命令失败",
  validation_failed: "配置缺失",
  error_unknown: "未知错误",
  unknown: "未知",
};

export const WAKEUP_CLIENT_VERSION_OPTIONS: { value: string; label: string }[] = [
  { value: "auto", label: "auto（跟随官方）" },
  { value: "official_stable", label: "official_stable（稳定）" },
  { value: "official_preview", label: "official_preview（预览）" },
  { value: "official_legacy", label: "official_legacy（兼容）" },
];

export type WakeupClientProfile = {
  requestedMode: string;
  fallbackMode: string;
  effectiveMode: string;
  runtimeArgs: string;
  gatewayMode: string;
  gatewayTransport: string;
  gatewayRouting: string;
  gatewayVersionHint: string;
  fallbackReason: string | null;
};

export type WakeupHistoryGroup = {
  runId: string;
  items: WakeupHistoryItem[];
  latest: WakeupHistoryItem;
  successCount: number;
  failedCount: number;
};

export const createWakeupTask = (): WakeupTask => ({
  id: `wakeup-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  name: "",
  enabled: true,
  account_id: "",
  trigger_mode: "cron",
  reset_window: "primary_window",
  window_day_policy: "all_days",
  window_fallback_policy: "none",
  client_version_mode: "auto",
  client_version_fallback_mode: "auto",
  command_template: "",
  model: "",
  prompt: "hi",
  cron: "0 */6 * * *",
  notes: "",
  timeout_seconds: 120,
  retry_failed_times: 1,
  pause_after_failures: 3,
  created_at: "",
  updated_at: "",
  last_run_at: null,
  last_status: null,
  last_category: null,
  last_message: null,
  consecutive_failures: 0,
});

export const getWakeupCategoryLabel = (category?: string | null) => {
  if (!category) return "未知";
  return WAKEUP_CATEGORY_LABELS[category] || category;
};

export const getWakeupCategoryTone = (category?: string | null) => {
  if (!category) return "muted";
  if (category === "success") return "success";
  if (category === "timeout" || category === "rate_limited") return "warning";
  return "danger";
};

export const normalizeClientVersionMode = (raw?: string | null) => {
  const value = String(raw || "").trim().toLowerCase();
  if (value === "official_stable" || value === "stable") return "official_stable";
  if (value === "official_preview" || value === "preview" || value === "beta") return "official_preview";
  if (value === "official_legacy" || value === "legacy" || value === "v1_legacy") return "official_legacy";
  return "auto";
};

const platformClientFamily = (originPlatform?: string | null) => {
  const platform = String(originPlatform || "").toLowerCase();
  if (platform.includes("gemini")) return "gemini";
  if (platform.includes("codex")) return "codex";
  return "generic";
};

const isModeSupportedForFamily = (family: string, mode: string) => {
  if (mode === "auto") return true;
  if (family === "gemini" || family === "codex") {
    return mode === "official_stable" || mode === "official_preview" || mode === "official_legacy";
  }
  return mode === "official_legacy";
};

const profileFieldsForMode = (family: string, mode: string) => {
  if (family === "gemini" && mode === "official_stable") {
    return {
      runtimeArgs: "--client-channel stable",
      gatewayMode: "strict",
      gatewayTransport: "oauth_refresh",
      gatewayRouting: "gemini_official",
      gatewayVersionHint: "Gemini 官方稳定通道",
    };
  }
  if (family === "gemini" && mode === "official_preview") {
    return {
      runtimeArgs: "--client-channel preview --enable-preview",
      gatewayMode: "compat_preview",
      gatewayTransport: "oauth_refresh",
      gatewayRouting: "gemini_preview",
      gatewayVersionHint: "Gemini 官方预览通道",
    };
  }
  if (family === "gemini" && mode === "official_legacy") {
    return {
      runtimeArgs: "--legacy-auth-flow",
      gatewayMode: "legacy_compat",
      gatewayTransport: "oauth_legacy",
      gatewayRouting: "gemini_legacy",
      gatewayVersionHint: "Gemini 旧版兼容链路",
    };
  }
  if (family === "codex" && mode === "official_stable") {
    return {
      runtimeArgs: "--channel stable",
      gatewayMode: "strict",
      gatewayTransport: "oauth_token",
      gatewayRouting: "codex_official",
      gatewayVersionHint: "Codex 官方稳定通道",
    };
  }
  if (family === "codex" && mode === "official_preview") {
    return {
      runtimeArgs: "--channel preview --enable-beta",
      gatewayMode: "compat_preview",
      gatewayTransport: "oauth_token",
      gatewayRouting: "codex_preview",
      gatewayVersionHint: "Codex 官方预览通道",
    };
  }
  if (mode === "official_legacy") {
    return {
      runtimeArgs: "--legacy-auth-flow",
      gatewayMode: "legacy_compat",
      gatewayTransport: "oauth_legacy",
      gatewayRouting: `${family}_legacy`,
      gatewayVersionHint: "通用旧版兼容链路",
    };
  }
  return {
    runtimeArgs: "",
    gatewayMode: "auto",
    gatewayTransport: "auto",
    gatewayRouting: "auto",
    gatewayVersionHint: "自动跟随当前官方客户端",
  };
};

export const resolveWakeupClientProfile = (
  originPlatform: string | undefined,
  modeRaw: string | undefined,
  fallbackRaw: string | undefined
): WakeupClientProfile => {
  const requestedMode = normalizeClientVersionMode(modeRaw);
  const fallbackMode = normalizeClientVersionMode(fallbackRaw);
  const family = platformClientFamily(originPlatform);
  let effectiveMode = requestedMode;
  let fallbackReason: string | null = null;

  if (!isModeSupportedForFamily(family, requestedMode)) {
    if (isModeSupportedForFamily(family, fallbackMode)) {
      effectiveMode = fallbackMode;
      fallbackReason = `平台 ${originPlatform || "unknown"} 不支持 ${requestedMode}，已回退到 ${fallbackMode}`;
    } else {
      effectiveMode = "auto";
      fallbackReason = `平台 ${originPlatform || "unknown"} 不支持 ${requestedMode} / ${fallbackMode}，已强制回退到 auto`;
    }
  }

  return {
    requestedMode,
    fallbackMode,
    effectiveMode,
    ...profileFieldsForMode(family, effectiveMode),
    fallbackReason,
  };
};

export const suggestWakeupCommandTemplate = (platform: string) => {
  const normalized = String(platform || "").toLowerCase();
  if (normalized === "gemini") {
    return 'gemini -m "{model}" -p "{prompt}"';
  }
  if (normalized === "codex") {
    return 'codex "{prompt}"';
  }
  if (normalized === "claude_code" || normalized === "antigravity") {
    return 'claude "{prompt}"';
  }
  return '"{prompt}"';
};

export const renderWakeupTaskCommandPreview = (
  task: WakeupTask,
  account?: Pick<IdeAccount, "email" | "origin_platform">
) => {
  const profile = resolveWakeupClientProfile(
    account?.origin_platform,
    task.client_version_mode,
    task.client_version_fallback_mode
  );
  const hadRuntimePlaceholder = (task.command_template || "").includes("{client_runtime_args}");
  const rendered = [
    ["{model}", task.model || ""],
    ["{prompt}", task.prompt || ""],
    ["{account_id}", task.account_id || ""],
    ["{email}", account?.email || ""],
    ["{client_version_mode}", profile.effectiveMode],
    ["{client_version_mode_requested}", profile.requestedMode],
    ["{client_version_fallback_mode}", profile.fallbackMode],
    ["{client_runtime_args}", profile.runtimeArgs],
    ["{gateway_mode}", profile.gatewayMode],
    ["{gateway_transport}", profile.gatewayTransport],
    ["{gateway_routing}", profile.gatewayRouting],
    ["{gateway_version_hint}", profile.gatewayVersionHint],
  ].reduce((command, [token, value]) => command.split(token).join(value), task.command_template || "");

  if (!hadRuntimePlaceholder && profile.runtimeArgs.trim()) {
    return `${rendered.trimEnd()} ${profile.runtimeArgs}`;
  }

  return rendered;
};

export const groupWakeupHistory = (history: WakeupHistoryItem[]): WakeupHistoryGroup[] => {
  const groups = new Map<string, WakeupHistoryItem[]>();

  for (const item of history) {
    const key = item.run_id || item.id;
    const bucket = groups.get(key) ?? [];
    bucket.push(item);
    groups.set(key, bucket);
  }

  return Array.from(groups.entries())
    .map(([runId, items]) => {
      const sorted = [...items].sort((a, b) => b.created_at.localeCompare(a.created_at));
      const latest = sorted[0];
      const successCount = sorted.filter((item) => item.status === "success").length;
      const failedCount = sorted.length - successCount;
      return { runId, items: sorted, latest, successCount, failedCount };
    })
    .sort((a, b) => b.latest.created_at.localeCompare(a.latest.created_at));
};
