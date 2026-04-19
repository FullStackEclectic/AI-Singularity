import { useEffect, useMemo, useState } from "react";
import {
  api,
  type WakeupHistoryItem,
  type WakeupState,
  type WakeupTask,
  type WakeupVerificationBatchResult,
} from "../../lib/api";
import type { AccountGroup, IdeAccount } from "../../types";
import {
  createWakeupTask,
  groupWakeupHistory,
  renderWakeupTaskCommandPreview,
  WAKEUP_UNGROUPED_FILTER,
} from "./wakeupUtils";

export function useWakeupPageState() {
  const [state, setState] = useState<WakeupState>({ enabled: false, tasks: [] });
  const [history, setHistory] = useState<WakeupHistoryItem[]>([]);
  const [accounts, setAccounts] = useState<IdeAccount[]>([]);
  const [accountGroups, setAccountGroups] = useState<AccountGroup[]>([]);
  const [taskGroupFilters, setTaskGroupFilters] = useState<Record<string, string>>({});
  const [message, setMessage] = useState("");
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [verificationSelection, setVerificationSelection] = useState<string[]>([]);
  const [verificationModel, setVerificationModel] = useState("");
  const [verificationPrompt, setVerificationPrompt] = useState("hi");
  const [verificationCommandTemplate, setVerificationCommandTemplate] = useState(
    'gemini -m "{model}" -p "{prompt}"'
  );
  const [verificationTimeout, setVerificationTimeout] = useState(120);
  const [verificationRetryFailedTimes, setVerificationRetryFailedTimes] = useState(1);
  const [verificationRunning, setVerificationRunning] = useState(false);
  const [verificationShowFailedOnly, setVerificationShowFailedOnly] = useState(false);
  const [verificationGroupFilter, setVerificationGroupFilter] = useState("all");
  const [verificationResultGroupFilter, setVerificationResultGroupFilter] = useState("all");
  const [verificationResult, setVerificationResult] =
    useState<WakeupVerificationBatchResult | null>(null);
  const [activeVerificationRunId, setActiveVerificationRunId] = useState<string | null>(null);
  const [runningTaskId, setRunningTaskId] = useState<string | null>(null);
  const [historyGroupFilter, setHistoryGroupFilter] = useState("all");

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

        if (cancelled) return;

        setState(wakeupState);
        setHistory(wakeupHistory);
        setAccounts(ideAccounts);
        setAccountGroups(groups);
      } catch (error) {
        if (!cancelled) {
          setMessage("加载 Wakeup 数据失败: " + String(error));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    load();

    return () => {
      cancelled = true;
    };
  }, []);

  const activeTasks = useMemo(
    () => state.tasks.filter((task) => task.enabled),
    [state.tasks]
  );

  const failingTasks = useMemo(
    () => state.tasks.filter((task) => (task.consecutive_failures || 0) > 0),
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
    if (filter === WAKEUP_UNGROUPED_FILTER) return !groupId;
    return groupId === filter;
  };

  const verificationGroupOptions = useMemo(() => {
    const usedGroupIds = new Set(
      accounts
        .map((account) => accountGroupByAccountId.get(account.id)?.id)
        .filter(Boolean)
    );

    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [accounts, accountGroups, accountGroupByAccountId]);

  const visibleVerificationAccounts = useMemo(
    () =>
      accounts.filter((account) =>
        matchesAccountGroupFilter(account.id, verificationGroupFilter)
      ),
    [accounts, verificationGroupFilter, accountGroupByAccountId]
  );

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

  const filteredHistory = useMemo(
    () =>
      history.filter((item) =>
        matchesAccountGroupFilter(item.account_id, historyGroupFilter)
      ),
    [history, historyGroupFilter, accountGroupByAccountId]
  );

  const groupedHistory = useMemo(
    () => groupWakeupHistory(filteredHistory),
    [filteredHistory]
  );

  useEffect(() => {
    if (
      verificationGroupFilter !== "all" &&
      verificationGroupFilter !== WAKEUP_UNGROUPED_FILTER &&
      !accountGroups.some((group) => group.id === verificationGroupFilter)
    ) {
      setVerificationGroupFilter("all");
    }
  }, [verificationGroupFilter, accountGroups]);

  useEffect(() => {
    if (
      verificationResultGroupFilter !== "all" &&
      verificationResultGroupFilter !== WAKEUP_UNGROUPED_FILTER &&
      !verificationResultGroupOptions.some(
        (group) => group.id === verificationResultGroupFilter
      )
    ) {
      setVerificationResultGroupFilter("all");
    }
  }, [verificationResultGroupFilter, verificationResultGroupOptions]);

  useEffect(() => {
    if (
      historyGroupFilter !== "all" &&
      historyGroupFilter !== WAKEUP_UNGROUPED_FILTER &&
      !historyGroupOptions.some((group) => group.id === historyGroupFilter)
    ) {
      setHistoryGroupFilter("all");
    }
  }, [historyGroupFilter, historyGroupOptions]);

  const saveState = async (next: WakeupState, successMessage = "Wakeup 状态已保存") => {
    setSaving(true);
    try {
      const saved = await api.wakeup.saveState(next);
      setState(saved);
      setMessage(successMessage);
    } catch (error) {
      setMessage("保存 Wakeup 状态失败: " + String(error));
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

  const handleAddTask = () => {
    const task = createWakeupTask();
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
    } catch (error) {
      setMessage("立即执行任务失败: " + String(error));
    } finally {
      setRunningTaskId(null);
    }
  };

  const handleCopyTaskCommand = async (task: WakeupTask) => {
    const account = accounts.find((item) => item.id === task.account_id);
    const rendered = renderWakeupTaskCommandPreview(task, account).trim();

    if (!rendered) {
      setMessage("当前任务还没有可复制的命令模板");
      return;
    }

    try {
      await navigator.clipboard.writeText(rendered);
      setMessage("已复制当前任务的渲染命令");
    } catch (error) {
      setMessage("复制任务命令失败: " + String(error));
    }
  };

  const clearHistory = async () => {
    try {
      await api.wakeup.clearHistory();
      setHistory([]);
      setMessage("Wakeup 历史已清空");
    } catch (error) {
      setMessage("清空 Wakeup 历史失败: " + String(error));
    }
  };

  const toggleVerificationAccount = (accountId: string) => {
    setVerificationSelection((prev) =>
      prev.includes(accountId)
        ? prev.filter((item) => item !== accountId)
        : [...prev, accountId]
    );
  };

  const runVerification = async (overrideAccountIds?: string[]) => {
    const targets =
      overrideAccountIds && overrideAccountIds.length > 0
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
    const runId = `verification-${Date.now()}-${Math.random()
      .toString(36)
      .slice(2, 8)}`;
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
    } catch (error) {
      setMessage("批次验证失败: " + String(error));
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
    } catch (error) {
      setMessage("取消批次验证失败: " + String(error));
    }
  };

  return {
    state,
    setState,
    history,
    accounts,
    taskGroupFilters,
    message,
    saving,
    loading,
    verificationSelection,
    setVerificationSelection,
    verificationModel,
    setVerificationModel,
    verificationPrompt,
    setVerificationPrompt,
    verificationCommandTemplate,
    setVerificationCommandTemplate,
    verificationTimeout,
    setVerificationTimeout,
    verificationRetryFailedTimes,
    setVerificationRetryFailedTimes,
    verificationRunning,
    verificationShowFailedOnly,
    setVerificationShowFailedOnly,
    verificationGroupFilter,
    setVerificationGroupFilter,
    verificationResultGroupFilter,
    setVerificationResultGroupFilter,
    verificationResult,
    activeVerificationRunId,
    runningTaskId,
    historyGroupFilter,
    setHistoryGroupFilter,
    activeTasks,
    failingTasks,
    accountGroupByAccountId,
    verificationGroupOptions,
    visibleVerificationAccounts,
    verificationResultGroupOptions,
    hasUngroupedVerificationResult,
    verificationVisibleItems,
    verificationFailedAccountIds,
    historyGroupOptions,
    hasUngroupedHistory,
    groupedHistory,
    getAccountGroupLabel,
    saveState,
    handleTaskChange,
    handleTaskGroupFilterChange,
    handleAddTask,
    handleDeleteTask,
    handleRunTaskNow,
    handleCopyTaskCommand,
    clearHistory,
    toggleVerificationAccount,
    runVerification,
    rerunFailedVerification,
    cancelVerification,
  };
}
