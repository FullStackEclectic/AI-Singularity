import { useEffect, useMemo, useState } from "react";
import { Play, Save, Trash2, History, Clock3 } from "lucide-react";
import {
  api,
  type WakeupHistoryItem,
  type WakeupState,
  type WakeupTask,
  type WakeupVerificationBatchResult,
} from "../../lib/api";
import type { AccountGroup } from "../../types";
import "./WakeupPage.css";

const createTask = (): WakeupTask => ({
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

const getWakeupCategoryLabel = (category?: string | null) => {
  if (!category) return "未知";
  return WAKEUP_CATEGORY_LABELS[category] || category;
};

const getWakeupCategoryTone = (category?: string | null) => {
  if (!category) return "muted";
  if (category === "success") return "success";
  if (category === "timeout" || category === "rate_limited") return "warning";
  return "danger";
};

const WAKEUP_CLIENT_VERSION_OPTIONS: { value: string; label: string }[] = [
  { value: "auto", label: "auto（跟随官方）" },
  { value: "official_stable", label: "official_stable（稳定）" },
  { value: "official_preview", label: "official_preview（预览）" },
  { value: "official_legacy", label: "official_legacy（兼容）" },
];

type WakeupClientProfile = {
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

const normalizeClientVersionMode = (raw?: string | null) => {
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

const resolveWakeupClientProfile = (
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

export default function WakeupPage() {
  const [state, setState] = useState<WakeupState>({ enabled: false, tasks: [] });
  const [history, setHistory] = useState<WakeupHistoryItem[]>([]);
  const [accounts, setAccounts] = useState<any[]>([]);
  const [accountGroups, setAccountGroups] = useState<AccountGroup[]>([]);
  const [taskGroupFilters, setTaskGroupFilters] = useState<Record<string, string>>({});
  const [message, setMessage] = useState("");
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [verificationSelection, setVerificationSelection] = useState<string[]>([]);
  const [verificationModel, setVerificationModel] = useState("");
  const [verificationPrompt, setVerificationPrompt] = useState("hi");
  const [verificationCommandTemplate, setVerificationCommandTemplate] = useState('gemini -m "{model}" -p "{prompt}"');
  const [verificationTimeout, setVerificationTimeout] = useState(120);
  const [verificationRetryFailedTimes, setVerificationRetryFailedTimes] = useState(1);
  const [verificationRunning, setVerificationRunning] = useState(false);
  const [verificationShowFailedOnly, setVerificationShowFailedOnly] = useState(false);
  const [verificationGroupFilter, setVerificationGroupFilter] = useState<string>("all");
  const [verificationResultGroupFilter, setVerificationResultGroupFilter] = useState<string>("all");
  const [verificationResult, setVerificationResult] = useState<WakeupVerificationBatchResult | null>(null);
  const [activeVerificationRunId, setActiveVerificationRunId] = useState<string | null>(null);
  const [runningTaskId, setRunningTaskId] = useState<string | null>(null);
  const [historyGroupFilter, setHistoryGroupFilter] = useState<string>("all");

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      setLoading(true);
      try {
        const [wakeupState, wakeupHistory, ideAccounts] = await Promise.all([
          api.wakeup.getState(),
          api.wakeup.loadHistory(),
          api.ideAccounts.list(),
        ]);
        const groups = await api.ideAccounts.listGroups().catch(() => []);
        if (!cancelled) {
          setState(wakeupState);
          setHistory(wakeupHistory);
          setAccounts(ideAccounts);
          setAccountGroups(groups);
        }
      } catch (e) {
        if (!cancelled) setMessage("加载 Wakeup 数据失败: " + String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (verificationGroupFilter === "all" || verificationGroupFilter === "__ungrouped__") return;
    if (!accountGroups.some((group) => group.id === verificationGroupFilter)) {
      setVerificationGroupFilter("all");
    }
  }, [verificationGroupFilter, accountGroups]);

  const activeTasks = useMemo(
    () => state.tasks.filter((task) => task.enabled),
    [state.tasks]
  );

  const accountGroupByAccountId = useMemo(() => {
    const map = new Map<string, AccountGroup>();
    for (const group of accountGroups) {
      for (const accountId of group.account_ids || []) {
        map.set(accountId, group);
      }
    }
    return map;
  }, [accountGroups]);

  const getAccountGroupLabel = (accountId?: string) => {
    if (!accountId) return "未分组";
    return accountGroupByAccountId.get(accountId)?.name || "未分组";
  };

  const matchesAccountGroupFilter = (accountId: string | undefined, filter: string) => {
    if (filter === "all") return true;
    const groupId = accountId ? accountGroupByAccountId.get(accountId)?.id : undefined;
    if (filter === "__ungrouped__") return !groupId;
    return groupId === filter;
  };

  const verificationGroupOptions = useMemo(() => {
    const usedGroupIds = new Set(accounts.map((account) => accountGroupByAccountId.get(account.id)?.id).filter(Boolean));
    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [accounts, accountGroups, accountGroupByAccountId]);

  const visibleVerificationAccounts = useMemo(() => {
    return accounts.filter((account) => matchesAccountGroupFilter(account.id, verificationGroupFilter));
  }, [accounts, accountGroupByAccountId, verificationGroupFilter]);

  const verificationResultGroupOptions = useMemo(() => {
    const usedGroupIds = new Set(
      (verificationResult?.items || [])
        .map((item) => accountGroupByAccountId.get(item.account_id)?.id)
        .filter(Boolean)
    );
    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [verificationResult, accountGroups, accountGroupByAccountId]);

  const hasUngroupedVerificationResult = useMemo(
    () =>
      (verificationResult?.items || []).some(
        (item) => !accountGroupByAccountId.get(item.account_id)?.id
      ),
    [verificationResult, accountGroupByAccountId]
  );

  const failingTasks = useMemo(
    () => state.tasks.filter((task) => (task.consecutive_failures || 0) > 0),
    [state.tasks]
  );

  const historyGroupOptions = useMemo(() => {
    const usedGroupIds = new Set(
      history
        .map((item) => accountGroupByAccountId.get(item.account_id)?.id)
        .filter(Boolean)
    );
    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [history, accountGroups, accountGroupByAccountId]);

  const hasUngroupedHistory = useMemo(
    () => history.some((item) => !accountGroupByAccountId.get(item.account_id)?.id),
    [history, accountGroupByAccountId]
  );

  useEffect(() => {
    if (verificationResultGroupFilter === "all" || verificationResultGroupFilter === "__ungrouped__") return;
    if (!verificationResultGroupOptions.some((group) => group.id === verificationResultGroupFilter)) {
      setVerificationResultGroupFilter("all");
    }
  }, [verificationResultGroupFilter, verificationResultGroupOptions]);

  useEffect(() => {
    if (historyGroupFilter === "all" || historyGroupFilter === "__ungrouped__") return;
    if (!historyGroupOptions.some((group) => group.id === historyGroupFilter)) {
      setHistoryGroupFilter("all");
    }
  }, [historyGroupFilter, historyGroupOptions]);

  const saveState = async (next: WakeupState, successMessage = "Wakeup 状态已保存") => {
    setSaving(true);
    try {
      const saved = await api.wakeup.saveState(next);
      setState(saved);
      setMessage(successMessage);
    } catch (e) {
      setMessage("保存 Wakeup 状态失败: " + String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleTaskChange = (id: string, patch: Partial<WakeupTask>) => {
    setState((prev) => ({
      ...prev,
      tasks: prev.tasks.map((task) => (task.id === id ? { ...task, ...patch } : task)),
    }));
  };

  const handleTaskGroupFilterChange = (taskId: string, value: string) => {
    setTaskGroupFilters((prev) => ({ ...prev, [taskId]: value }));
  };

  const suggestCommandTemplate = (platform: string) => {
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

  const handleAddTask = () => {
    const task = createTask();
    setTaskGroupFilters((prev) => ({ ...prev, [task.id]: "all" }));
    setState((prev) => ({ ...prev, tasks: [task, ...prev.tasks] }));
  };

  const handleDeleteTask = (id: string) => {
    setTaskGroupFilters((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
    setState((prev) => ({ ...prev, tasks: prev.tasks.filter((task) => task.id !== id) }));
  };

  const handleRunTaskNow = async (taskId: string) => {
    setRunningTaskId(taskId);
    try {
      const nextState = await api.wakeup.runTaskNow(taskId);
      setState(nextState);
      setHistory(await api.wakeup.loadHistory());
      const task = nextState.tasks.find((item) => item.id === taskId);
      setMessage(task?.last_message || "任务已执行");
    } catch (e) {
      setMessage("立即执行任务失败: " + String(e));
    } finally {
      setRunningTaskId(null);
    }
  };

  const renderTaskCommandPreview = (task: WakeupTask) => {
    const account = accounts.find((item) => item.id === task.account_id);
    const email = account?.email || "";
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
      ["{email}", email],
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

  const handleCopyTaskCommand = async (task: WakeupTask) => {
    const rendered = renderTaskCommandPreview(task).trim();
    if (!rendered) {
      setMessage("当前任务还没有可复制的命令模板");
      return;
    }
    try {
      await navigator.clipboard.writeText(rendered);
      setMessage("已复制当前任务的渲染命令");
    } catch (e) {
      setMessage("复制任务命令失败: " + String(e));
    }
  };

  const clearHistory = async () => {
    try {
      await api.wakeup.clearHistory();
      setHistory([]);
      setMessage("Wakeup 历史已清空");
    } catch (e) {
      setMessage("清空 Wakeup 历史失败: " + String(e));
    }
  };

  const toggleVerificationAccount = (accountId: string) => {
    setVerificationSelection((prev) =>
      prev.includes(accountId) ? prev.filter((item) => item !== accountId) : [...prev, accountId]
    );
  };

  const verificationGroupFilteredItems = useMemo(
    () =>
      (verificationResult?.items || []).filter((item) =>
        matchesAccountGroupFilter(item.account_id, verificationResultGroupFilter)
      ),
    [verificationResult, verificationResultGroupFilter, accountGroupByAccountId]
  );

  const verificationVisibleItems = useMemo(
    () =>
      verificationShowFailedOnly
        ? verificationGroupFilteredItems.filter((item) => item.status !== "success")
        : verificationGroupFilteredItems,
    [verificationGroupFilteredItems, verificationShowFailedOnly]
  );

  const verificationFailedAccountIds = useMemo(
    () =>
      verificationGroupFilteredItems
        .filter((item) => item.status !== "success")
        .map((item) => item.account_id),
    [verificationGroupFilteredItems]
  );

  const runVerification = async (overrideAccountIds?: string[]) => {
    const targets = overrideAccountIds && overrideAccountIds.length > 0
      ? overrideAccountIds
      : verificationSelection;
    if (targets.length === 0) {
      setMessage("请至少选择一个账号进行批次验证");
      return;
    }
    if (!verificationModel.trim() || !verificationCommandTemplate.trim()) {
      setMessage("请填写验证模型和命令模板");
      return;
    }

    setVerificationRunning(true);
    const runId = `verification-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setActiveVerificationRunId(runId);
    try {
      const result = await api.wakeup.runVerificationBatch({
        accountIds: targets,
        model: verificationModel.trim(),
        prompt: verificationPrompt,
        commandTemplate: verificationCommandTemplate.trim(),
        timeoutSeconds: verificationTimeout,
        retryFailedTimes: verificationRetryFailedTimes,
        runId,
      });
      setVerificationResult(result);
      setVerificationShowFailedOnly(false);
      setVerificationResultGroupFilter("all");
      setHistory(await api.wakeup.loadHistory());
      setMessage(
        result.canceled
          ? `批次验证已取消：成功 ${result.success_count}，失败 ${result.failed_count}`
          : `批次验证完成：成功 ${result.success_count}，失败 ${result.failed_count}，重试 ${result.retried_count} 次`
      );
    } catch (e) {
      setMessage("批次验证失败: " + String(e));
    } finally {
      setVerificationRunning(false);
      setActiveVerificationRunId(null);
    }
  };

  const rerunFailedVerification = async () => {
    if (verificationFailedAccountIds.length === 0) {
      setMessage("当前没有失败账号可重试");
      return;
    }
    setVerificationSelection(verificationFailedAccountIds);
    await runVerification(verificationFailedAccountIds);
  };

  const cancelVerification = async () => {
    if (!activeVerificationRunId) {
      setMessage("当前没有可取消的批次验证");
      return;
    }
    try {
      const canceled = await api.wakeup.cancelVerificationRun(activeVerificationRunId);
      setMessage(canceled ? "已请求取消当前批次验证" : "当前批次验证已经结束");
    } catch (e) {
      setMessage("取消批次验证失败: " + String(e));
    }
  };

  const filteredHistory = useMemo(
    () => history.filter((item) => matchesAccountGroupFilter(item.account_id, historyGroupFilter)),
    [history, historyGroupFilter, accountGroupByAccountId]
  );

  const groupedHistory = useMemo(() => {
    const groups = new Map<string, WakeupHistoryItem[]>();
    for (const item of filteredHistory) {
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
  }, [filteredHistory]);

  return (
    <div className="wakeup-page">
      <div className="page-header">
        <div>
          <h1 className="page-title"><Clock3 size={22} className="text-primary" /> Wakeup / Verification</h1>
          <p className="page-subtitle">任务、历史、cron 调度、即时执行和批次验证已接入。本页已支持结果分类统计与失败账号重试。</p>
        </div>
        <div className="wakeup-header-actions">
          <button className="btn btn-secondary" onClick={handleAddTask}>新增任务</button>
          <button className="btn btn-primary" onClick={() => saveState(state)} disabled={saving}>
            <Save size={14} /> {saving ? "保存中..." : "保存全部"}
          </button>
        </div>
      </div>

      {message && <div className="wakeup-message">{message}</div>}

      <div className="wakeup-overview">
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">总任务数</div>
          <div className="wakeup-stat-value">{state.tasks.length}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">启用任务</div>
          <div className="wakeup-stat-value">{activeTasks.length}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">历史记录</div>
          <div className="wakeup-stat-value">{history.length}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">连续失败</div>
          <div className="wakeup-stat-value">{failingTasks.length}</div>
        </div>
      </div>

      <div className="card wakeup-global-card">
        <label className="wakeup-toggle">
          <input
            type="checkbox"
            checked={state.enabled}
            onChange={(e) => setState((prev) => ({ ...prev, enabled: e.target.checked }))}
          />
          <span>启用 Wakeup 总开关</span>
        </label>
        <div className="wakeup-hint">
          当前已接入基础 cron 调度器。命中任务时会自动写入历史记录，并支持任务级失败重试、连续失败计数和自动暂停。
        </div>
      </div>

      <div className="wakeup-layout">
        <section className="card wakeup-task-panel">
          <div className="wakeup-section-title">任务列表</div>
          {loading ? (
            <div className="wakeup-empty">加载中...</div>
          ) : state.tasks.length === 0 ? (
            <div className="wakeup-empty">当前还没有 Wakeup 任务</div>
          ) : (
            <div className="wakeup-task-list">
              {state.tasks.map((task) => (
                <div key={task.id} className="wakeup-task-item">
                  {(() => {
                    const taskGroupFilter = taskGroupFilters[task.id] || "all";
                    const taskVisibleAccounts = taskGroupFilter === "all"
                      ? accounts
                      : taskGroupFilter === "__ungrouped__"
                        ? accounts.filter((account) => !accountGroupByAccountId.get(account.id))
                        : accounts.filter((account) => accountGroupByAccountId.get(account.id)?.id === taskGroupFilter);
                    const selectedAccount = accounts.find((account) => account.id === task.account_id);
                    const taskClientProfile = resolveWakeupClientProfile(
                      selectedAccount?.origin_platform,
                      task.client_version_mode,
                      task.client_version_fallback_mode
                    );
                    return (
                      <>
                  <div className="wakeup-task-grid">
                    <label>
                      <span>任务名</span>
                      <input className="form-input" value={task.name} onChange={(e) => handleTaskChange(task.id, { name: e.target.value })} />
                    </label>
                    <label>
                      <span>账号分组</span>
                      <select
                        className="form-input"
                        value={taskGroupFilter}
                        onChange={(e) => handleTaskGroupFilterChange(task.id, e.target.value)}
                      >
                        <option value="all">全部账号</option>
                        <option value="__ungrouped__">未分组</option>
                        {verificationGroupOptions.map((group) => (
                          <option key={group.id} value={group.id}>
                            {group.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>账号</span>
                      <select
                        className="form-input"
                        value={task.account_id}
                        onChange={(e) => {
                          const accountId = e.target.value;
                          const account = accounts.find((item) => item.id === accountId);
                          handleTaskChange(task.id, {
                            account_id: accountId,
                            command_template: task.command_template.trim()
                              ? task.command_template
                              : suggestCommandTemplate(account?.origin_platform),
                          });
                        }}
                      >
                        <option value="">选择 IDE 账号</option>
                        {taskVisibleAccounts.map((account) => (
                          <option key={account.id} value={account.id}>
                            {(account.label || account.email) + " · " + account.origin_platform + " · " + getAccountGroupLabel(account.id)}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>触发模式</span>
                      <select
                        className="form-input"
                        value={task.trigger_mode || "cron"}
                        onChange={(e) => handleTaskChange(task.id, { trigger_mode: e.target.value })}
                      >
                        <option value="cron">Cron</option>
                        <option value="quota_reset">Quota Reset</option>
                      </select>
                    </label>
                    {(task.trigger_mode || "cron") === "quota_reset" ? (
                      <>
                        <label>
                          <span>重置窗口</span>
                          <select
                            className="form-input"
                            value={task.reset_window || "primary_window"}
                            onChange={(e) => handleTaskChange(task.id, { reset_window: e.target.value })}
                          >
                            <option value="primary_window">primary_window</option>
                            <option value="secondary_window">secondary_window</option>
                            <option value="either_window">either_window</option>
                          </select>
                        </label>
                        <label>
                          <span>窗口日期策略</span>
                          <select
                            className="form-input"
                            value={task.window_day_policy || "all_days"}
                            onChange={(e) => handleTaskChange(task.id, { window_day_policy: e.target.value })}
                          >
                            <option value="all_days">任意日</option>
                            <option value="workdays">仅工作日</option>
                            <option value="weekends">仅周末</option>
                          </select>
                        </label>
                        <label>
                          <span>窗口回退策略</span>
                          <select
                            className="form-input"
                            value={task.window_fallback_policy || "none"}
                            onChange={(e) => handleTaskChange(task.id, { window_fallback_policy: e.target.value })}
                          >
                            <option value="none">不回退</option>
                            <option value="primary_then_secondary_on_failure">主窗口失败后回退次窗口</option>
                          </select>
                        </label>
                      </>
                    ) : null}
                    <label>
                      <span>客户端版本模式</span>
                      <select
                        className="form-input"
                        value={normalizeClientVersionMode(task.client_version_mode)}
                        onChange={(e) => handleTaskChange(task.id, { client_version_mode: e.target.value })}
                      >
                        {WAKEUP_CLIENT_VERSION_OPTIONS.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>模式回退策略</span>
                      <select
                        className="form-input"
                        value={normalizeClientVersionMode(task.client_version_fallback_mode)}
                        onChange={(e) =>
                          handleTaskChange(task.id, { client_version_fallback_mode: e.target.value })
                        }
                      >
                        <option value="auto">auto（推荐）</option>
                        <option value="official_stable">official_stable</option>
                        <option value="official_preview">official_preview</option>
                        <option value="official_legacy">official_legacy</option>
                      </select>
                    </label>
                    <label>
                      <span>模型</span>
                      <input className="form-input" value={task.model} onChange={(e) => handleTaskChange(task.id, { model: e.target.value })} placeholder="例如 gpt-4.1 / gemini-2.5-pro" />
                    </label>
                    {(task.trigger_mode || "cron") === "cron" ? (
                      <label>
                        <span>Cron</span>
                        <input className="form-input" value={task.cron} onChange={(e) => handleTaskChange(task.id, { cron: e.target.value })} />
                      </label>
                    ) : (
                      <label>
                        <span>重置来源说明</span>
                        <input className="form-input" value="从账号 quota_json 读取 reset_time" disabled />
                      </label>
                    )}
                    <label className="span-2">
                      <span>执行命令模板</span>
                      <input
                        className="form-input"
                        value={task.command_template}
                        onChange={(e) => handleTaskChange(task.id, { command_template: e.target.value })}
                        placeholder='例如 gemini -m "{model}" -p "{prompt}"'
                      />
                    </label>
                    <label className="span-2">
                      <span>Prompt</span>
                      <textarea className="form-input wakeup-textarea" value={task.prompt} onChange={(e) => handleTaskChange(task.id, { prompt: e.target.value })} />
                    </label>
                    <label>
                      <span>超时秒数</span>
                      <input
                        className="form-input"
                        type="number"
                        min={10}
                        max={3600}
                        value={task.timeout_seconds}
                        onChange={(e) => handleTaskChange(task.id, { timeout_seconds: Math.max(10, Number(e.target.value) || 120) })}
                      />
                    </label>
                    <label>
                      <span>失败重试</span>
                      <input
                        className="form-input"
                        type="number"
                        min={0}
                        max={5}
                        value={task.retry_failed_times ?? 0}
                        onChange={(e) => handleTaskChange(task.id, { retry_failed_times: Math.max(0, Math.min(5, Number(e.target.value) || 0)) })}
                      />
                    </label>
                    <label>
                      <span>失败后自动暂停</span>
                      <input
                        className="form-input"
                        type="number"
                        min={0}
                        max={20}
                        value={task.pause_after_failures ?? 0}
                        onChange={(e) => handleTaskChange(task.id, { pause_after_failures: Math.max(0, Math.min(20, Number(e.target.value) || 0)) })}
                      />
                    </label>
                    <label className="span-2">
                      <span>备注</span>
                      <input className="form-input" value={task.notes || ""} onChange={(e) => handleTaskChange(task.id, { notes: e.target.value })} />
                    </label>
                  </div>
                  <div className="wakeup-task-footer">
                    <label className="wakeup-toggle">
                      <input
                        type="checkbox"
                        checked={task.enabled}
                        onChange={(e) => handleTaskChange(task.id, { enabled: e.target.checked })}
                      />
                      <span>启用任务</span>
                    </label>
                    <div className="wakeup-task-template-hint">
                      可用占位符：{"{model}"}、{"{prompt}"}、{"{account_id}"}、{"{email}"}、{"{client_runtime_args}"}、{"{gateway_mode}"}、{"{gateway_transport}"}、{"{gateway_routing}"}
                    </div>
                    <div className="wakeup-task-template-hint">
                      当前客户端模式：{taskClientProfile.effectiveMode} · 参数：{taskClientProfile.runtimeArgs || "无"} · 网关：{taskClientProfile.gatewayMode}/{taskClientProfile.gatewayTransport}
                      {taskClientProfile.fallbackReason ? ` · ${taskClientProfile.fallbackReason}` : ""}
                    </div>
                    <div className="wakeup-task-actions">
                      <button className="btn btn-primary btn-sm" onClick={() => handleRunTaskNow(task.id)} disabled={runningTaskId === task.id}>
                        <Play size={14} /> {runningTaskId === task.id ? "执行中..." : "立即执行"}
                      </button>
                      <button className="btn btn-secondary btn-sm" onClick={() => handleCopyTaskCommand(task)}>
                        <Play size={14} /> 复制渲染命令
                      </button>
                      <button className="btn btn-danger-ghost btn-sm" onClick={() => handleDeleteTask(task.id)}>
                        <Trash2 size={14} /> 删除
                      </button>
                    </div>
                  </div>
                  {(task.last_run_at || task.last_status || task.last_message) && (
                    <div className="wakeup-history-message" style={{ marginTop: 10 }}>
                      最近结果：
                      {task.last_status ? ` [${task.last_status}]` : ""}
                      {task.last_run_at ? ` ${new Date(task.last_run_at).toLocaleString()}` : ""}
                      {task.last_message ? ` · ${task.last_message}` : ""}
                      {task.last_category ? ` · 分类 ${getWakeupCategoryLabel(task.last_category)}` : ""}
                      {(task.consecutive_failures || 0) > 0 ? ` · 连续失败 ${task.consecutive_failures} 次` : ""}
                      {task.account_id ? ` · 分组 ${getAccountGroupLabel(task.account_id)}` : ""}
                      {` · 客户端模式 ${normalizeClientVersionMode(task.client_version_mode)} -> ${normalizeClientVersionMode(task.client_version_fallback_mode)}`}
                      {task.trigger_mode === "quota_reset"
                        ? ` · 触发 quota ${task.reset_window || "primary_window"} · ${task.window_day_policy || "all_days"} · ${task.window_fallback_policy || "none"}`
                        : ""}
                    </div>
                  )}
                      </>
                    );
                  })()}
                </div>
              ))}
            </div>
          )}
        </section>

        <div className="wakeup-side-stack">
          <section className="card wakeup-history-panel">
            <div className="wakeup-history-header">
              <div className="wakeup-section-title"><Play size={16} /> 批次验证</div>
              <div className="wakeup-header-actions">
                <button className="btn btn-primary btn-sm" onClick={() => runVerification()} disabled={verificationRunning}>
                  {verificationRunning ? "执行中..." : "开始验证"}
                </button>
                <button
                  className="btn btn-danger-ghost btn-sm"
                  onClick={cancelVerification}
                  disabled={!verificationRunning || !activeVerificationRunId}
                >
                  取消当前批次
                </button>
              </div>
            </div>
            <div className="wakeup-verification-config">
              <label>
                <span>验证模型</span>
                <input className="form-input" value={verificationModel} onChange={(e) => setVerificationModel(e.target.value)} placeholder="例如 gemini-2.5-pro" />
              </label>
              <label>
                <span>Prompt</span>
                <textarea className="form-input wakeup-textarea" value={verificationPrompt} onChange={(e) => setVerificationPrompt(e.target.value)} />
              </label>
              <label>
                <span>命令模板</span>
                <input className="form-input" value={verificationCommandTemplate} onChange={(e) => setVerificationCommandTemplate(e.target.value)} />
              </label>
              <label>
                <span>超时秒数</span>
                <input className="form-input" type="number" min={10} max={3600} value={verificationTimeout} onChange={(e) => setVerificationTimeout(Math.max(10, Number(e.target.value) || 120))} />
              </label>
              <label>
                <span>失败重试次数</span>
                <input className="form-input" type="number" min={0} max={5} value={verificationRetryFailedTimes} onChange={(e) => setVerificationRetryFailedTimes(Math.max(0, Math.min(5, Number(e.target.value) || 0)))} />
              </label>
            </div>
            <div className="wakeup-verification-accounts">
              <div className="wakeup-group-filters">
                <button
                  className={`btn btn-secondary btn-sm ${verificationGroupFilter === "all" ? "active" : ""}`}
                  onClick={() => setVerificationGroupFilter("all")}
                  type="button"
                >
                  全部分组
                </button>
                <button
                  className={`btn btn-secondary btn-sm ${verificationGroupFilter === "__ungrouped__" ? "active" : ""}`}
                  onClick={() => setVerificationGroupFilter("__ungrouped__")}
                  type="button"
                >
                  未分组
                </button>
                {verificationGroupOptions.map((group) => (
                  <button
                    key={group.id}
                    className={`btn btn-secondary btn-sm ${verificationGroupFilter === group.id ? "active" : ""}`}
                    onClick={() => setVerificationGroupFilter(group.id)}
                    type="button"
                  >
                    {group.name}
                  </button>
                ))}
                <button
                  className="btn btn-secondary btn-sm"
                  type="button"
                  disabled={visibleVerificationAccounts.length === 0}
                  onClick={() =>
                    setVerificationSelection((prev) => [
                      ...new Set([...prev, ...visibleVerificationAccounts.map((account) => account.id)]),
                    ])
                  }
                >
                  只选当前分组
                </button>
              </div>
              {accounts.length === 0 ? (
                <div className="wakeup-empty">当前没有可验证的 IDE 账号</div>
              ) : visibleVerificationAccounts.length === 0 ? (
                <div className="wakeup-empty">当前分组下没有可验证账号</div>
              ) : (
                visibleVerificationAccounts.map((account) => (
                  <label key={account.id} className="wakeup-verification-account">
                    <input
                      type="checkbox"
                      checked={verificationSelection.includes(account.id)}
                      onChange={() => toggleVerificationAccount(account.id)}
                    />
                    <span>{(account.label || account.email) + " · " + account.origin_platform + " · " + getAccountGroupLabel(account.id)}</span>
                  </label>
                ))
              )}
            </div>
            {verificationResult && (
              <div className="wakeup-verification-result">
                <div className="wakeup-history-top">
                  <strong>本次结果</strong>
                  <span>执行 {verificationResult.executed_count} 个</span>
                </div>
                <div className="wakeup-history-meta">
                  <span>成功：{verificationResult.success_count}</span>
                  <span>失败：{verificationResult.failed_count}</span>
                  <span>重试：{verificationResult.retried_count}</span>
                </div>
                <div className="wakeup-category-grid">
                  {verificationResult.category_counts.map((item) => (
                    <div key={item.category} className="wakeup-category-chip">
                      <span>{getWakeupCategoryLabel(item.category)}</span>
                      <strong>{item.count}</strong>
                    </div>
                  ))}
                </div>
                <div className="wakeup-history-header" style={{ marginTop: 8 }}>
                  <div className="wakeup-history-meta">
                    <span>结果列表</span>
                    <span>当前显示：{verificationVisibleItems.length}</span>
                  </div>
                  <div className="wakeup-header-actions">
                    <button
                      className="btn btn-secondary btn-sm"
                      onClick={() => setVerificationShowFailedOnly((prev) => !prev)}
                    >
                      {verificationShowFailedOnly ? "显示全部" : "仅看失败"}
                    </button>
                    <button
                      className="btn btn-primary btn-sm"
                      onClick={rerunFailedVerification}
                      disabled={verificationRunning || verificationFailedAccountIds.length === 0}
                    >
                      重试失败账号
                    </button>
                  </div>
                </div>
                <div className="wakeup-group-filters">
                  <button
                    className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === "all" ? "active" : ""}`}
                    onClick={() => setVerificationResultGroupFilter("all")}
                    type="button"
                  >
                    全部分组
                  </button>
                  {hasUngroupedVerificationResult && (
                    <button
                      className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === "__ungrouped__" ? "active" : ""}`}
                      onClick={() => setVerificationResultGroupFilter("__ungrouped__")}
                      type="button"
                    >
                      未分组
                    </button>
                  )}
                  {verificationResultGroupOptions.map((group) => (
                    <button
                      key={group.id}
                      className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === group.id ? "active" : ""}`}
                      onClick={() => setVerificationResultGroupFilter(group.id)}
                      type="button"
                    >
                      {group.name}
                    </button>
                  ))}
                </div>
                <div className="wakeup-history-list">
                  {verificationVisibleItems.length === 0 ? (
                    <div className="wakeup-empty">当前分组下没有匹配的验证结果</div>
                  ) : (
                    verificationVisibleItems.map((item) => (
                      <div key={item.account_id} className="wakeup-history-item">
                        <div className="wakeup-history-top">
                          <strong>{item.email}</strong>
                          <span>{item.status}</span>
                        </div>
                        <div className="wakeup-history-meta">
                          <span>分类：{getWakeupCategoryLabel(item.category)}</span>
                          <span>尝试：{item.attempts}</span>
                          <span>分组：{getAccountGroupLabel(item.account_id)}</span>
                        </div>
                        <div className="wakeup-history-message">{item.message}</div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}
          </section>

          <section className="card wakeup-history-panel">
            <div className="wakeup-history-header">
              <div className="wakeup-section-title"><History size={16} /> 执行历史</div>
              <button className="btn btn-danger-ghost btn-sm" onClick={clearHistory}>清空历史</button>
            </div>
            <div className="wakeup-group-filters">
              <button
                className={`btn btn-secondary btn-sm ${historyGroupFilter === "all" ? "active" : ""}`}
                onClick={() => setHistoryGroupFilter("all")}
                type="button"
              >
                全部分组
              </button>
              {hasUngroupedHistory && (
                <button
                  className={`btn btn-secondary btn-sm ${historyGroupFilter === "__ungrouped__" ? "active" : ""}`}
                  onClick={() => setHistoryGroupFilter("__ungrouped__")}
                  type="button"
                >
                  未分组
                </button>
              )}
              {historyGroupOptions.map((group) => (
                <button
                  key={group.id}
                  className={`btn btn-secondary btn-sm ${historyGroupFilter === group.id ? "active" : ""}`}
                  onClick={() => setHistoryGroupFilter(group.id)}
                  type="button"
                >
                  {group.name}
                </button>
              ))}
            </div>
            {history.length === 0 ? (
              <div className="wakeup-empty">当前还没有 Wakeup 历史</div>
            ) : groupedHistory.length === 0 ? (
              <div className="wakeup-empty">当前分组下没有 Wakeup 历史</div>
            ) : (
              <div className="wakeup-history-list">
                {groupedHistory.map((group) => (
                  <div key={group.runId} className="wakeup-history-item">
                    <div className="wakeup-history-top">
                      <strong>{group.latest.task_name}</strong>
                      <span>{new Date(group.latest.created_at).toLocaleString()}</span>
                    </div>
                    <div className="wakeup-history-meta">
                      <span>本批次 {group.items.length} 条</span>
                      <span>成功：{group.successCount}</span>
                      <span>失败：{group.failedCount}</span>
                      <span>模型：{group.latest.model || "—"}</span>
                    </div>
                    <div className="wakeup-history-list" style={{ marginTop: 10 }}>
                      {group.items.map((item) => (
                        <div key={item.id} className="wakeup-history-item wakeup-history-subitem">
                          <div className="wakeup-history-top">
                            <strong>{item.account_id || "—"}</strong>
                            <span>{item.status}</span>
                          </div>
                          <div className="wakeup-history-meta">
                            <span>分类：{getWakeupCategoryLabel(item.category)}</span>
                            <span className={`wakeup-category-badge ${getWakeupCategoryTone(item.category)}`}>
                              {getWakeupCategoryLabel(item.category)}
                            </span>
                            <span>分组：{getAccountGroupLabel(item.account_id)}</span>
                          </div>
                          {item.message && <div className="wakeup-history-message">{item.message}</div>}
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
