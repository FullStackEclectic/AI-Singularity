import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { RefreshCw, Cpu, Activity, Skull, Folder } from "lucide-react";
import { api, type CurrentAccountSnapshot } from "../../lib/api";
import { ExpandedMessageModal } from "./ExpandedMessageModal";
import { RelatedGeminiTranscriptsDialog } from "./RelatedGeminiTranscriptsDialog";
import { SessionDetailPane } from "./SessionDetailPane";
import { SessionsFiltersPanel } from "./SessionsFiltersPanel";
import { SessionGroupsList } from "./SessionGroupsList";
import type {
  ActionMessage,
  ChatMessage,
  ChatSession,
  CodexInstanceRecord,
  CodexSettingsDialogState,
  ConfirmDialogState,
  ProviderOption,
  SessionGroup,
  ZombieProcess,
} from "./sessionTypes";
import {
  buildResumeCommandWithCwd,
  formatCwdLabel,
  formatDate,
  formatMessageTime,
  formatUptime,
  getGeminiToolOutputDir,
  getResumeCommand,
  getSessionCwd,
  getSessionFlags,
  isNoTranscriptSession,
  isProblemSession,
  isToolRelatedMessage,
  isWorkspaceHistorySession,
} from "./sessionUtils";
import type { AccountGroup, IdeAccount } from "../../types";
import "./SessionsPage.css";

export default function SessionsPage() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [zombies, setZombies] = useState<ZombieProcess[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [messageViewMode, setMessageViewMode] = useState<"all" | "tool">("all");
  const [sessionSignalFilter, setSessionSignalFilter] = useState<"all" | "tool" | "log" | "failed_tool">("all");
  const [expandedMessage, setExpandedMessage] = useState<ChatMessage | null>(null);
  const [showRelatedGeminiDialog, setShowRelatedGeminiDialog] = useState(false);
  const [relatedSearchQuery, setRelatedSearchQuery] = useState("");
  const [relatedStatusFilter, setRelatedStatusFilter] = useState<"all" | "success" | "failed" | "none">("all");
  const [collapsedToolKeys, setCollapsedToolKeys] = useState<string[]>([]);
  const [expandedGroups, setExpandedGroups] = useState<string[]>([]);
  const [selectedFilepaths, setSelectedFilepaths] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [toolFilter, setToolFilter] = useState<string>("all");
  const [sourceFilter, setSourceFilter] = useState<"all" | "transcript" | "workspace_history" | "no_transcript">("all");
  const [problemAccountGroupFilter, setProblemAccountGroupFilter] = useState<string>("all");
  const [codexInstances, setCodexInstances] = useState<CodexInstanceRecord[]>([]);
  const [defaultCodexInstance, setDefaultCodexInstance] = useState<CodexInstanceRecord | null>(null);
  const [codexProviders, setCodexProviders] = useState<ProviderOption[]>([]);
  const [ideAccounts, setIdeAccounts] = useState<IdeAccount[]>([]);
  const [accountGroups, setAccountGroups] = useState<AccountGroup[]>([]);
  const [currentSnapshots, setCurrentSnapshots] = useState<CurrentAccountSnapshot[]>([]);
  const [showCodexInstances, setShowCodexInstances] = useState(false);
  const [codexInstanceName, setCodexInstanceName] = useState("");
  const [codexInstanceDir, setCodexInstanceDir] = useState("");
  const [codexInstanceLoading, setCodexInstanceLoading] = useState(false);
  const [sharedSyncBusyInstanceId, setSharedSyncBusyInstanceId] = useState<string | null>(null);
  const [actionMessages, setActionMessages] = useState<(ActionMessage & { id: string; createdAt: number })[]>([]);
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null);
  const [confirmDialogBusy, setConfirmDialogBusy] = useState(false);
  const [codexSettingsDialog, setCodexSettingsDialog] = useState<CodexSettingsDialogState | null>(null);
  const codexInstanceCount = codexInstances.length + 1;

  const pushActionMessage = (message: ActionMessage) => {
    setActionMessages((prev) => [
      {
        ...message,
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        createdAt: Date.now(),
      },
      ...prev,
    ].slice(0, 6));
  };

  const clearActionMessages = () => setActionMessages([]);

  const fetchAll = async () => {
    setLoading(true);
    try {
      const [sessData, zombieData, codexData, defaultCodex, ideAccountsList, groups, snapshots] = await Promise.all([
        invoke<ChatSession[]>("list_sessions"),
        invoke<ZombieProcess[]>("scan_zombies"),
        invoke<CodexInstanceRecord[]>("list_codex_instances"),
        invoke<CodexInstanceRecord>("get_default_codex_instance"),
        api.ideAccounts.list(),
        api.ideAccounts.listGroups().catch(() => []),
        api.providerCurrent.listSnapshots(),
      ]);
      const providerData = await invoke<ProviderOption[]>("get_providers");
      setSessions(sessData);
      setSelectedFilepaths((prev) => prev.filter((filepath) => sessData.some((item) => item.filepath === filepath)));
      setZombies(zombieData);
      setCodexInstances(codexData);
      setDefaultCodexInstance(defaultCodex);
      setIdeAccounts(ideAccountsList);
      setAccountGroups(groups);
      setCurrentSnapshots(snapshots);
      setCodexProviders(providerData.filter((item) => {
        try {
          const targets = item.tool_targets ? JSON.parse(item.tool_targets) as string[] : ["claude_code"];
          return targets.includes("codex");
        } catch {
          return false;
        }
      }));
    } catch (e) {
      console.error("Failed to load sessions/zombies:", e);
      pushActionMessage({ text: "加载会话数据失败：" + String(e), tone: "error" });
    } finally {
      setLoading(false);
    }
  };

  const loadSession = async (session: ChatSession) => {
    setSelectedSession(session);
    setMessageViewMode("all");
    setShowRelatedGeminiDialog(false);
    setRelatedSearchQuery("");
    setRelatedStatusFilter("all");
    setCollapsedToolKeys([]);
    setMessagesLoading(true);
    try {
      const msgData = await invoke<ChatMessage[]>("get_session_details", { filepath: session.filepath });
      setMessages(msgData);
    } catch (e) {
      console.error("Failed to load session details:", e);
      setMessages([]);
    } finally {
      setMessagesLoading(false);
    }
  };

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 10000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (problemAccountGroupFilter === "all" || problemAccountGroupFilter === "__ungrouped__") return;
    if (!accountGroups.some((group) => group.id === problemAccountGroupFilter)) {
      setProblemAccountGroupFilter("all");
    }
  }, [accountGroups, problemAccountGroupFilter]);

  const handleCopyCmd = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    const cmd = getResumeCommand(session);
    const fullCmd = cwd ? `cd /d "${cwd}" && ${cmd}` : cmd;
    navigator.clipboard.writeText(fullCmd).then(() => pushActionMessage({ text: "恢复指令已复制到剪贴板", tone: "success" }));
  };

  const handleCopyDir = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    if (cwd) {
       navigator.clipboard.writeText(cwd).then(() => pushActionMessage({ text: "目录路径已复制", tone: "success" }));
    } else {
       pushActionMessage({ text: "当前会话没有可推断的工作目录，请改用复制恢复命令。", tone: "info" });
    }
  };

  const handleLaunchTerminal = async (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    const cmd = getResumeCommand(session);
    
    try {
      await invoke("launch_session_terminal", { cwd: cwd || ".", command: cmd });
      pushActionMessage({ text: "已尝试在外部终端启动会话", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "外置终端下发执行失败：" + String(e), tone: "error" });
    }
  };

  const accountGroupByAccountId = useMemo(() => {
    const map = new Map<string, AccountGroup>();
    for (const group of accountGroups) {
      for (const accountId of group.account_ids || []) {
        map.set(accountId, group);
      }
    }
    return map;
  }, [accountGroups]);

  const allCodexInstances = useMemo(
    () => [...(defaultCodexInstance ? [defaultCodexInstance] : []), ...codexInstances],
    [defaultCodexInstance, codexInstances]
  );

  const codexInstanceById = useMemo(
    () => new Map(allCodexInstances.map((item) => [item.id, item])),
    [allCodexInstances]
  );

  const currentAccountIdByPlatform = useMemo(() => {
    const map = new Map<string, string>();
    for (const snapshot of currentSnapshots) {
      const platform = String(snapshot.platform || "").trim().toLowerCase();
      const accountId = snapshot.account_id?.trim();
      if (platform && accountId) {
        map.set(platform, accountId);
      }
    }
    return map;
  }, [currentSnapshots]);

  const getSessionAccountId = (session: ChatSession): string | null => {
    const tool = (session.tool_type || "").trim().toLowerCase();
    if (tool === "codex") {
      const instance =
        (session.instance_id ? codexInstanceById.get(session.instance_id) : undefined) ??
        allCodexInstances.find((item) => item.name === session.instance_name) ??
        defaultCodexInstance ??
        null;
      if (instance) {
        if (instance.is_default && instance.follow_local_account) {
          return currentAccountIdByPlatform.get("codex") ?? null;
        }
        return instance.bind_account_id || null;
      }
      return currentAccountIdByPlatform.get("codex") ?? null;
    }
    if (tool === "geminicli") {
      return currentAccountIdByPlatform.get("gemini") ?? null;
    }
    return null;
  };

  const getSessionAccountGroupId = (session: ChatSession): string | null => {
    const accountId = getSessionAccountId(session);
    if (!accountId) return null;
    return accountGroupByAccountId.get(accountId)?.id ?? null;
  };

  const matchesSessionFiltersWithoutProblemGroup = (session: ChatSession, normalizedQuery: string) => {
    if (toolFilter !== "all" && (session.tool_type || "Unknown") !== toolFilter) {
      return false;
    }
    if (sessionSignalFilter === "tool" && !session.has_tool_calls) {
      return false;
    }
    if (sessionSignalFilter === "log" && !session.has_log_events) {
      return false;
    }
    if (
      sessionSignalFilter === "failed_tool" &&
      (!session.latest_tool_status || session.latest_tool_status === "success")
    ) {
      return false;
    }
    if (sourceFilter === "workspace_history" && session.source_kind !== "workspace_history") {
      return false;
    }
    if (sourceFilter === "transcript" && session.messages_count === 0) {
      return false;
    }
    if (sourceFilter === "no_transcript" && session.messages_count > 0) {
      return false;
    }
    if (normalizedQuery) {
      const haystack = [
        session.title,
        session.filepath,
        session.cwd,
        session.tool_type,
        session.instance_name,
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      if (!haystack.includes(normalizedQuery)) {
        return false;
      }
    }
    return true;
  };

  const sessionGroups = useMemo<SessionGroup[]>(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();
    const grouped = new Map<string, ChatSession[]>();

    for (const session of sessions) {
      if (!matchesSessionFiltersWithoutProblemGroup(session, normalizedQuery)) continue;
      if (problemAccountGroupFilter !== "all" && isProblemSession(session)) {
        const groupId = getSessionAccountGroupId(session);
        if (problemAccountGroupFilter === "__ungrouped__") {
          if (groupId) continue;
        } else if (groupId !== problemAccountGroupFilter) {
          continue;
        }
      }
      const cwd = getSessionCwd(session) || `[${session.tool_type || "Unknown"}]`;
      const bucket = grouped.get(cwd) ?? [];
      bucket.push(session);
      grouped.set(cwd, bucket);
    }

    return Array.from(grouped.entries())
      .map(([cwd, groupedSessions]) => {
        const normalized = cwd.replace(/\\/g, "/").replace(/\/$/, "");
        const parts = normalized.split("/").filter(Boolean);
        const label = cwd.startsWith("[") ? cwd : (parts[parts.length - 1] || cwd);
        const sortedSessions = [...groupedSessions].sort((a, b) => b.updated_at - a.updated_at);
        return {
          cwd,
          label,
          updated_at: sortedSessions[0]?.updated_at ?? 0,
          sessions: sortedSessions,
        };
      })
      .sort((a, b) => b.updated_at - a.updated_at || a.label.localeCompare(b.label, "zh-CN"));
  }, [
    sessions,
    searchQuery,
    toolFilter,
    sourceFilter,
    sessionSignalFilter,
    problemAccountGroupFilter,
    accountGroupByAccountId,
    allCodexInstances,
    codexInstanceById,
    currentAccountIdByPlatform,
    defaultCodexInstance,
  ]);

  const toggleGroupExpanded = (cwd: string) => {
    setExpandedGroups((prev) =>
      prev.includes(cwd) ? prev.filter((item) => item !== cwd) : [...prev, cwd]
    );
  };

  const toggleSessionSelected = (filepath: string) => {
    setSelectedFilepaths((prev) =>
      prev.includes(filepath) ? prev.filter((item) => item !== filepath) : [...prev, filepath]
    );
  };

  const toggleAllSessionsSelected = () => {
    if (selectedFilepaths.length === sessions.length) {
      setSelectedFilepaths([]);
    } else {
      setSelectedFilepaths(sessions.map((item) => item.filepath));
    }
  };

  const toggleGroupSelected = (group: SessionGroup) => {
    const allSelected = group.sessions.every((session) => selectedFilepaths.includes(session.filepath));
    setSelectedFilepaths((prev) => {
      const next = new Set(prev);
      if (allSelected) {
        group.sessions.forEach((session) => next.delete(session.filepath));
      } else {
        group.sessions.forEach((session) => next.add(session.filepath));
      }
      return Array.from(next);
    });
  };

  const handleMoveToTrash = async () => {
    if (selectedFilepaths.length === 0) {
      pushActionMessage({ text: "请至少选择一条会话", tone: "error" });
      return;
    }
    setConfirmDialog({
      title: "移到废纸篓",
      description: `确认将选中的 ${selectedFilepaths.length} 条会话移到废纸篓吗？`,
      confirmLabel: "确认移动",
      tone: "danger",
      action: async () => {
        try {
          const result = await invoke<{ message: string }>("move_sessions_to_trash", { filepaths: selectedFilepaths });
          pushActionMessage({ text: result.message, tone: "success" });
          setSelectedFilepaths([]);
          if (selectedSession && selectedFilepaths.includes(selectedSession.filepath)) {
            setSelectedSession(null);
            setMessages([]);
          }
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "移动会话失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const handleRepairCodexIndex = async () => {
    try {
      const result = await invoke<{ message: string }>("repair_codex_session_index");
      pushActionMessage({ text: result.message, tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "修复 Codex 会话索引失败：" + String(e), tone: "error" });
    }
  };

  const handleSyncCodexThreads = async () => {
    try {
      const result = await invoke<{ message: string }>("sync_codex_threads_across_instances");
      pushActionMessage({ text: result.message, tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "同步 Codex 线程失败：" + String(e), tone: "error" });
    }
  };

  const handlePickCodexDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择 Codex 实例目录",
    });
    if (typeof selected === "string") {
      setCodexInstanceDir(selected);
    }
  };

  const handleAddCodexInstance = async () => {
    if (!codexInstanceName.trim() || !codexInstanceDir.trim()) {
      pushActionMessage({ text: "请填写实例名称并选择目录", tone: "error" });
      return;
    }
    setCodexInstanceLoading(true);
    pushActionMessage({ text: "正在同步共享资源并添加 Codex 实例...", tone: "info" });
    try {
      const added = await invoke<CodexInstanceRecord>("add_codex_instance", {
        name: codexInstanceName.trim(),
        userDataDir: codexInstanceDir.trim(),
      });
      setCodexInstanceName("");
      setCodexInstanceDir("");
      if (added.has_shared_conflicts) {
        pushActionMessage({
          text: `Codex 实例已添加，共享资源同步完成（发现 ${added.shared_conflict_paths?.length || 0} 项冲突并已跳过）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: "Codex 实例已添加，共享资源同步完成", tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "添加 Codex 实例失败（共享资源同步或实例登记失败）：" + String(e), tone: "error" });
    } finally {
      setCodexInstanceLoading(false);
    }
  };

  const handleDeleteCodexInstance = async (id: string) => {
    setConfirmDialog({
      title: "删除 Codex 实例",
      description: "确认删除这个 Codex 实例目录吗？不会删除真实文件，只会移除索引。",
      confirmLabel: "删除",
      tone: "danger",
      action: async () => {
        try {
          await invoke("delete_codex_instance", { id });
          pushActionMessage({ text: "Codex 实例已删除", tone: "success" });
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "删除 Codex 实例失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const handleUpdateCodexInstanceSettings = async (instance: CodexInstanceRecord) => {
    setCodexSettingsDialog({
      instance,
      extraArgs: instance.extra_args || "",
      bindAccountId: instance.bind_account_id || "",
      bindProviderId: instance.bind_provider_id || "",
      followLocalAccount: !!instance.follow_local_account,
    });
  };

  const handleStartCodexInstance = async (id: string) => {
    setSharedSyncBusyInstanceId(id);
    pushActionMessage({ text: "正在同步共享资源并启动 Codex 实例...", tone: "info" });
    try {
      const started = await invoke<CodexInstanceRecord>("start_codex_instance", { id });
      if (started.has_shared_conflicts) {
        pushActionMessage({
          text: `Codex 实例已启动，共享资源同步完成（发现 ${started.shared_conflict_paths?.length || 0} 项冲突并已跳过）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: "Codex 实例已启动，共享资源同步完成", tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "启动 Codex 实例失败（共享资源同步或启动失败）：" + String(e), tone: "error" });
    } finally {
      setSharedSyncBusyInstanceId(null);
    }
  };

  const handleSyncCodexSharedResources = async (instance: CodexInstanceRecord) => {
    setSharedSyncBusyInstanceId(instance.id);
    pushActionMessage({ text: `正在重试共享资源同步：${instance.name}`, tone: "info" });
    try {
      const synced = await invoke<CodexInstanceRecord>("sync_codex_instance_shared_resources", { id: instance.id });
      if (synced.has_shared_conflicts) {
        pushActionMessage({
          text: `${instance.name} 共享资源同步完成（仍有 ${synced.shared_conflict_paths?.length || 0} 项冲突）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: `${instance.name} 共享资源同步完成`, tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: `${instance.name} 共享资源同步失败：${String(e)}`, tone: "error" });
    } finally {
      setSharedSyncBusyInstanceId(null);
    }
  };

  const handleStopCodexInstance = async (id: string) => {
    try {
      await invoke("stop_codex_instance", { id });
      pushActionMessage({ text: "Codex 实例已停止", tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "停止 Codex 实例失败：" + String(e), tone: "error" });
    }
  };

  const handleOpenCodexWindow = async (id: string) => {
    try {
      await invoke("open_codex_instance_window", { id });
      pushActionMessage({ text: "已尝试切换到 Codex 实例窗口", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "切换 Codex 实例窗口失败：" + String(e), tone: "error" });
    }
  };

  const handleCreateInstanceFloatingCard = async (instance: CodexInstanceRecord) => {
    try {
      await api.floatingCards.create({
        scope: "instance",
        instance_id: instance.id,
        title: `${instance.name} 账号浮窗`,
        bound_platforms: ["codex"],
        window_label: "main",
      });
      pushActionMessage({ text: `${instance.name} 的浮动账号卡片已创建`, tone: "success" });
    } catch (e) {
      pushActionMessage({ text: `创建实例浮窗失败：${String(e)}`, tone: "error" });
    }
  };

  const handleCloseAllCodexInstances = async () => {
    setConfirmDialog({
      title: "关闭全部 Codex 实例",
      description: "确认关闭所有 Codex 实例吗？",
      confirmLabel: "全部关闭",
      tone: "danger",
      action: async () => {
        try {
          await invoke("close_all_codex_instances");
          pushActionMessage({ text: "已关闭全部 Codex 实例", tone: "success" });
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "关闭全部 Codex 实例失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const selectedSessions = useMemo(
    () => sessions.filter((item) => selectedFilepaths.includes(item.filepath)),
    [sessions, selectedFilepaths]
  );

  const visibleSessions = useMemo(
    () => sessionGroups.flatMap((group) => group.sessions),
    [sessionGroups]
  );

  const visibleSessionFilepaths = useMemo(
    () => visibleSessions.map((item) => item.filepath),
    [visibleSessions]
  );

  const visibleProblemSessions = useMemo(
    () => visibleSessions.filter((item) => isProblemSession(item)),
    [visibleSessions]
  );

  const visibleProblemFilepaths = useMemo(
    () => visibleProblemSessions.map((item) => item.filepath),
    [visibleProblemSessions]
  );

  const visibleProblemDirs = useMemo(
    () => [...new Set(visibleProblemSessions.map((item) => getSessionCwd(item)).filter((item) => !!item))],
    [visibleProblemSessions]
  );

  const visibleProblemResumeCommands = useMemo(
    () => visibleProblemSessions.map((item) => buildResumeCommandWithCwd(item)),
    [visibleProblemSessions]
  );

  const workspaceHistoryFilepaths = useMemo(
    () => visibleSessions.filter(isWorkspaceHistorySession).map((item) => item.filepath),
    [visibleSessions]
  );

  const noTranscriptFilepaths = useMemo(
    () => visibleSessions.filter(isNoTranscriptSession).map((item) => item.filepath),
    [visibleSessions]
  );

  const { options: problemAccountGroupOptions, hasUngroupedProblemSessions } = useMemo(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();
    const usedGroupIds = new Set<string>();
    let hasUngrouped = false;

    for (const session of sessions) {
      if (!isProblemSession(session)) continue;
      if (!matchesSessionFiltersWithoutProblemGroup(session, normalizedQuery)) continue;
      const groupId = getSessionAccountGroupId(session);
      if (groupId) {
        usedGroupIds.add(groupId);
      } else {
        hasUngrouped = true;
      }
    }

    const options = accountGroups.filter((group) => usedGroupIds.has(group.id));
    if (
      problemAccountGroupFilter !== "all" &&
      problemAccountGroupFilter !== "__ungrouped__" &&
      !options.some((group) => group.id === problemAccountGroupFilter)
    ) {
      const selected = accountGroups.find((group) => group.id === problemAccountGroupFilter);
      if (selected) {
        options.push(selected);
      }
    }

    return { options, hasUngroupedProblemSessions: hasUngrouped };
  }, [
    sessions,
    searchQuery,
    accountGroups,
    toolFilter,
    sourceFilter,
    sessionSignalFilter,
    problemAccountGroupFilter,
    accountGroupByAccountId,
    allCodexInstances,
    codexInstanceById,
    currentAccountIdByPlatform,
    defaultCodexInstance,
  ]);

  const toolFilterOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const session of sessions) {
      const key = session.tool_type || "Unknown";
      counts.set(key, (counts.get(key) || 0) + 1);
    }
    return Array.from(counts.entries())
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], "zh-CN"))
      .map(([tool, count]) => ({ tool, count }));
  }, [sessions]);

  const sessionOverview = useMemo(() => {
    const workspaceHistoryCount = sessions.filter((item) => item.source_kind === "workspace_history").length;
    const transcriptCount = sessions.filter((item) => item.messages_count > 0).length;
    const noTranscriptCount = sessions.length - transcriptCount;
    const visibleCount = sessionGroups.reduce((sum, group) => sum + group.sessions.length, 0);
    const visibleProblemCount = visibleSessions.filter(isProblemSession).length;
    return {
      total: sessions.length,
      visible: visibleCount,
      workspaceHistory: workspaceHistoryCount,
      noTranscript: noTranscriptCount,
      visibleProblem: visibleProblemCount,
    };
  }, [sessions, sessionGroups, visibleSessions]);

  const problemGroupFilterLabel = useMemo(() => {
    if (problemAccountGroupFilter === "all") return null;
    if (problemAccountGroupFilter === "__ungrouped__") return "未分组";
    return accountGroups.find((group) => group.id === problemAccountGroupFilter)?.name || "未知分组";
  }, [problemAccountGroupFilter, accountGroups]);

  const currentSessionViewLabel = useMemo(() => {
    let base = "全部会话";
    if (sessionSignalFilter === "tool") base = "含工具调用";
    else if (sessionSignalFilter === "log") base = "含日志事件";
    else if (sessionSignalFilter === "failed_tool") base = "最近工具失败";
    else if (sourceFilter === "workspace_history") base = "工作区历史";
    else if (sourceFilter === "no_transcript") base = "无转录";
    else if (sourceFilter === "transcript") base = "有转录";
    else if (searchQuery.trim()) base = "搜索结果";
    else if (toolFilter !== "all") base = `${toolFilter} 会话`;
    if (!problemGroupFilterLabel) return base;
    return `${base} · 问题分组 ${problemGroupFilterLabel}`;
  }, [sessionSignalFilter, sourceFilter, searchQuery, toolFilter, problemGroupFilterLabel]);

  const selectedGroupsCount = useMemo(
    () =>
      sessionGroups.filter((group) =>
        group.sessions.some((item) => selectedFilepaths.includes(item.filepath))
      ).length,
    [sessionGroups, selectedFilepaths]
  );

  const problemGroupCount = useMemo(
    () =>
      sessionGroups.filter((group) =>
        group.sessions.some((item) => isProblemSession(item))
      ).length,
    [sessionGroups]
  );

  const handleCopyText = (text: string, successText: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => pushActionMessage({ text: successText, tone: "success" }))
      .catch((e) => pushActionMessage({ text: "复制失败：" + String(e), tone: "error" }));
  };

  const handleCopyMessageBlock = (message: ChatMessage) => {
    const title = selectedSession?.title ? `[${selectedSession.title}]` : "";
    const stamp = message.timestamp ? ` ${formatMessageTime(message.timestamp)}` : "";
    handleCopyText(
      `${title} ${message.role.toUpperCase()}${stamp}\n\n${message.content}`.trim(),
      "当前消息块已复制"
    );
  };

  const handleCopyVisibleMessages = (onlyTool = false) => {
    const source = onlyTool ? visibleMessages.filter(isToolRelatedMessage) : visibleMessages;
    if (source.length === 0) {
      pushActionMessage({ text: onlyTool ? "当前没有可复制的工具调用摘要" : "当前没有可复制的消息", tone: "info" });
      return;
    }
    const text = source
      .map((message) => {
        const stamp = message.timestamp ? ` ${formatMessageTime(message.timestamp)}` : "";
        return `${message.role.toUpperCase()}${stamp}\n${message.content}`;
      })
      .join("\n\n---\n\n");
    handleCopyText(text, onlyTool ? "工具调用摘要已复制" : "当前可见消息已复制");
  };

  const handleOpenToolOutputDir = async () => {
    const dir = getGeminiToolOutputDir(selectedSession);
    if (!dir) {
      pushActionMessage({ text: "当前会话没有可推断的工具输出目录", tone: "info" });
      return;
    }
    try {
      await openPath(dir);
      pushActionMessage({ text: "已尝试打开工具输出目录", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "打开工具输出目录失败：" + String(e), tone: "error" });
    }
  };

  const handleOpenMessageSource = async (message: ChatMessage) => {
    if (!message.source_path) {
      pushActionMessage({ text: "当前消息没有可打开的源文件", tone: "info" });
      return;
    }
    try {
      await openPath(message.source_path);
      pushActionMessage({ text: "已尝试打开消息源文件", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "打开消息源文件失败：" + String(e), tone: "error" });
    }
  };

  const toggleToolMessageCollapsed = (key: string) => {
    setCollapsedToolKeys((prev) =>
      prev.includes(key) ? prev.filter((item) => item !== key) : [...prev, key]
    );
  };

  const relatedGeminiTranscripts = useMemo(() => {
    if (!selectedSession || selectedSession.tool_type !== "GeminiCLI") return [];
    const cwd = getSessionCwd(selectedSession);
    if (!cwd) return [];
    return sessions
      .filter((item) =>
        item.tool_type === "GeminiCLI" &&
        item.source_kind === "transcript" &&
        getSessionCwd(item) === cwd &&
        item.filepath !== selectedSession.filepath
      )
      .sort((a, b) => b.updated_at - a.updated_at);
  }, [selectedSession, sessions]);

  const filteredRelatedGeminiTranscripts = useMemo(() => {
    const normalizedQuery = relatedSearchQuery.trim().toLowerCase();
    return relatedGeminiTranscripts.filter((item) => {
      if (relatedStatusFilter === "success" && item.latest_tool_status !== "success") {
        return false;
      }
      if (
        relatedStatusFilter === "failed" &&
        (!item.latest_tool_status || item.latest_tool_status === "success")
      ) {
        return false;
      }
      if (relatedStatusFilter === "none" && item.latest_tool_status) {
        return false;
      }
      if (!normalizedQuery) {
        return true;
      }
      const haystack = [
        item.title,
        item.filepath,
        item.latest_tool_name,
        item.latest_tool_status,
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(normalizedQuery);
    });
  }, [relatedGeminiTranscripts, relatedSearchQuery, relatedStatusFilter]);

  const jumpToRelatedGeminiTranscript = async () => {
    const target = relatedGeminiTranscripts[0];
    if (!target) {
      pushActionMessage({ text: "当前工作区还没有可跳转的 Gemini 转录", tone: "info" });
      return;
    }
    await loadSession(target);
    pushActionMessage({ text: "已跳转到同工作区的最新 Gemini 转录", tone: "success" });
  };

  const handleMoveSpecificSessionsToTrash = async (filepaths: string[], label: string) => {
    if (filepaths.length === 0) {
      pushActionMessage({ text: `当前没有可处理的${label}`, tone: "info" });
      return;
    }
    setConfirmDialog({
      title: "移到废纸篓",
      description: `确认将当前${label}中的 ${filepaths.length} 条会话移到废纸篓吗？`,
      confirmLabel: "确认移动",
      tone: "danger",
      action: async () => {
        try {
          const result = await invoke<{ message: string }>("move_sessions_to_trash", { filepaths });
          pushActionMessage({ text: result.message, tone: "success" });
          setSelectedFilepaths((prev) => prev.filter((item) => !filepaths.includes(item)));
          if (selectedSession && filepaths.includes(selectedSession.filepath)) {
            setSelectedSession(null);
            setMessages([]);
          }
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: `移动${label}失败：` + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const handleLaunchProblemSessionsInTerminal = async (limit = 3) => {
    if (visibleProblemSessions.length === 0) {
      pushActionMessage({ text: "当前没有可拉起的问题会话", tone: "info" });
      return;
    }
    const targets = visibleProblemSessions.slice(0, limit);
    let success = 0;
    for (const session of targets) {
      const cwd = getSessionCwd(session);
      const cmd = getResumeCommand(session);
      try {
        await invoke("launch_session_terminal", { cwd: cwd || ".", command: cmd });
        success += 1;
      } catch (e) {
        pushActionMessage({ text: `拉起会话失败：${session.title} · ${String(e)}`, tone: "error" });
      }
    }
    const suffix =
      visibleProblemSessions.length > limit
        ? `（为避免终端风暴，已限制为前 ${limit} 条）`
        : "";
    pushActionMessage({
      text: `已尝试拉起 ${success}/${targets.length} 条问题会话${suffix}`,
      tone: success > 0 ? "success" : "error",
    });
  };

  const codexInstanceCards = useMemo(() => {
    return allCodexInstances.map((instance) => {
      const sessionCount = sessions.filter((session) => {
        if (session.tool_type !== "Codex") return false;
        if (instance.is_default) {
          return !session.instance_name || session.instance_name === instance.name;
        }
        return session.instance_name === instance.name;
      }).length;
      return { ...instance, sessionCount };
    });
  }, [allCodexInstances, sessions]);

  const runningCodexInstances = useMemo(
    () => codexInstanceCards.filter((item) => item.running).length,
    [codexInstanceCards]
  );

  const codexAccounts = useMemo(
    () => ideAccounts.filter((item) => item.origin_platform === "codex"),
    [ideAccounts]
  );
  const currentCodexAccountId = useMemo(
    () => currentSnapshots.find((item: CurrentAccountSnapshot) => item.platform === "codex")?.account_id ?? null,
    [currentSnapshots]
  );
  const getCodexAccountLabel = (accountId?: string | null) => {
    if (!accountId) return "未绑定账号";
    const matched = codexAccounts.find((item) => item.id === accountId);
    if (!matched) return accountId;
    return matched.label?.trim() || matched.email;
  };
  const getEffectiveCodexAccountId = (instance: CodexInstanceRecord) =>
    instance.is_default && instance.follow_local_account
      ? currentCodexAccountId
      : instance.bind_account_id || null;

  const visibleMessages = useMemo(
    () =>
      messageViewMode === "tool"
        ? messages.filter((message) => isToolRelatedMessage(message))
        : messages,
    [messageViewMode, messages]
  );

  const selectedSessionToolOutputDir = useMemo(
    () => getGeminiToolOutputDir(selectedSession),
    [selectedSession]
  );

  return (
    <div className="sessions-page cyberpunk-theme">
      {/* 侧边栏结构 */}
      <div className="sessions-sidebar cyber-sidebar">
        <div className="sessions-header">
          <h2 className="cyber-title-sm">
            <Activity size={16} className="pulse-icon text-accent" /> ZOMBIE_RADAR // 全域劫持雷达
          </h2>
          <div className="sessions-header-actions">
            <button
              className="cyber-icon-btn"
              onClick={handleRepairCodexIndex}
              title="修复 Codex 会话索引"
            >
              <Folder size={14} />
            </button>
            <button
              className="cyber-icon-btn"
              onClick={handleSyncCodexThreads}
              title="同步 Codex 缺失线程"
              disabled={codexInstanceCount < 2}
            >
              <RefreshCw size={14} />
            </button>
            <button
              className="cyber-icon-btn"
              onClick={() => setShowCodexInstances(true)}
              title="管理 Codex 实例目录"
            >
              <Cpu size={14} />
            </button>
            <button
              className="cyber-icon-btn danger"
              onClick={handleMoveToTrash}
              disabled={selectedFilepaths.length === 0}
              title="将选中的会话移到废纸篓"
            >
              <Skull size={14} />
            </button>
            <button className="cyber-icon-btn" onClick={fetchAll} disabled={loading} title="刷新系统探针">
              <RefreshCw size={14} className={loading ? "spin" : ""} />
            </button>
          </div>
        </div>
        <SessionsFiltersPanel
          sessionOverview={sessionOverview}
          currentSessionViewLabel={currentSessionViewLabel}
          selectedFilepathsCount={selectedFilepaths.length}
          searchQuery={searchQuery}
          toolFilter={toolFilter}
          sourceFilter={sourceFilter}
          sessionSignalFilter={sessionSignalFilter}
          problemAccountGroupFilter={problemAccountGroupFilter}
          problemAccountGroupOptions={problemAccountGroupOptions}
          hasUngroupedProblemSessions={hasUngroupedProblemSessions}
          toolFilterOptions={toolFilterOptions}
          sessionGroups={sessionGroups}
          problemGroupCount={problemGroupCount}
          visibleSessionFilepaths={visibleSessionFilepaths}
          visibleProblemFilepaths={visibleProblemFilepaths}
          workspaceHistoryFilepaths={workspaceHistoryFilepaths}
          noTranscriptFilepaths={noTranscriptFilepaths}
          visibleProblemDirs={visibleProblemDirs}
          visibleProblemResumeCommands={visibleProblemResumeCommands}
          onSearchQueryChange={setSearchQuery}
          onToolFilterChange={setToolFilter}
          onSourceFilterChange={setSourceFilter}
          onSessionSignalFilterChange={setSessionSignalFilter}
          onProblemAccountGroupFilterChange={setProblemAccountGroupFilter}
          onExpandVisibleGroups={() => setExpandedGroups(sessionGroups.map((group) => group.cwd))}
          onExpandProblemGroups={() =>
            setExpandedGroups(
              sessionGroups
                .filter((group) => group.sessions.some((item) => isProblemSession(item)))
                .map((group) => group.cwd)
            )
          }
          onCollapseAllGroups={() => setExpandedGroups([])}
          onSelectVisibleSessions={() => setSelectedFilepaths((prev) => [...new Set([...prev, ...visibleSessionFilepaths])])}
          onSelectProblemSessions={() => setSelectedFilepaths((prev) => [...new Set([...prev, ...visibleProblemFilepaths])])}
          onSelectWorkspaceHistory={() => setSelectedFilepaths((prev) => [...new Set([...prev, ...workspaceHistoryFilepaths])])}
          onSelectNoTranscript={() => setSelectedFilepaths((prev) => [...new Set([...prev, ...noTranscriptFilepaths])])}
          onMoveProblemSessionsToTrash={() => handleMoveSpecificSessionsToTrash(visibleProblemFilepaths, "问题会话")}
          onCopyProblemDirs={() =>
            handleCopyText(
              visibleProblemDirs.join("\n"),
              `已复制 ${visibleProblemDirs.length} 条问题会话目录`
            )
          }
          onCopyProblemCommands={() =>
            handleCopyText(
              visibleProblemResumeCommands.join("\n"),
              `已复制 ${visibleProblemResumeCommands.length} 条问题会话恢复命令`
            )
          }
          onLaunchProblemSessions={() => handleLaunchProblemSessionsInTerminal(3)}
          onClearFilters={() => {
            setSourceFilter("all");
            setSessionSignalFilter("all");
            setToolFilter("all");
            setSearchQuery("");
            setProblemAccountGroupFilter("all");
          }}
        />
        {selectedFilepaths.length > 0 && (
          <div className="session-batch-bar">
            <div className="session-batch-title">批量处理队列</div>
            <div className="session-batch-meta">
              已选 {selectedSessions.length} 条会话，覆盖 {selectedGroupsCount} 个工作区
            </div>
            <div className="session-batch-actions">
              <button className="btn btn-ghost btn-xs" onClick={toggleAllSessionsSelected}>
                {selectedFilepaths.length === sessions.length ? "取消全选" : "全选全部"}
              </button>
              <button className="btn btn-ghost btn-xs" onClick={() => setSelectedFilepaths([])}>
                清空选择
              </button>
              <button className="btn btn-danger-ghost btn-xs" onClick={handleMoveToTrash}>
                移到废纸篓
              </button>
            </div>
          </div>
        )}

        {actionMessages.length > 0 && (
          <div className="session-action-stack">
            <div className="session-action-stack-header">
              <span>操作结果</span>
              <button className="btn btn-ghost btn-xs" onClick={clearActionMessages}>清空</button>
            </div>
            {actionMessages.map((item) => (
              <div key={item.id} className={`session-action-bar ${item.tone ?? "info"}`}>
                <div className="session-action-text">{item.text}</div>
                <div className="session-action-time">
                  {new Date(item.createdAt).toLocaleTimeString("zh-CN", {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  })}
                </div>
              </div>
            ))}
          </div>
        )}
        
        <div className="sessions-list">
          {/* Zombies Section */}
          <div className="section-divider">
            <span>[ 活跃宿主进程 ] ACTIVE_ZOMBIES</span>
          </div>
          
          {zombies.length === 0 && !loading && (
            <div className="empty-text">未探测到活动的受体进程</div>
          )}
          
          {zombies.map((z) => (
            <div key={z.pid} className="zombie-item">
              <div className="zombie-header">
                <span className="zombie-name"><Cpu size={12}/> {z.tool_type}</span>
                <span className="zombie-pid">PID: {z.pid}</span>
              </div>
              <div className="zombie-meta">
                <span title={z.cwd}>CWD: {z.cwd.length > 20 ? '...'+z.cwd.slice(-20) : z.cwd}</span>
                <span>UP: {formatUptime(z.active_time_sec)}</span>
              </div>
              <div className="zombie-actions">
                <button className="cyber-btn-mini toxic" onClick={() => pushActionMessage({ text: `功能研发中：自动修改 ${z.tool_type} 的路由并热重启进程`, tone: "info" })}>
                  <Skull size={10}/> 注入毒素代理
                </button>
              </div>
            </div>
          ))}

          <div className="section-divider mt-4">
            <span>[ CODEX 多实例 ] INSTANCE_MATRIX</span>
          </div>

          <div className="instance-overview-card">
            <div className="instance-overview-stats">
              <div className="instance-overview-stat">
                <span className="instance-overview-value">{codexInstanceCards.length}</span>
                <span className="instance-overview-label">实例</span>
              </div>
              <div className="instance-overview-stat">
                <span className="instance-overview-value">{runningCodexInstances}</span>
                <span className="instance-overview-label">运行中</span>
              </div>
              <div className="instance-overview-stat">
                <span className="instance-overview-value">
                  {sessions.filter((item) => item.tool_type === "Codex").length}
                </span>
                <span className="instance-overview-label">Codex 会话</span>
              </div>
            </div>
            <div className="instance-overview-list">
              {codexInstanceCards.map((item) => (
                <button
                  key={item.id}
                  className="instance-overview-item"
                  onClick={() => setShowCodexInstances(true)}
                  title={item.user_data_dir}
                >
                  <div className="instance-overview-item-main">
                    <span className="instance-overview-item-name">
                      {item.name}
                      {item.is_default ? " · 默认" : ""}
                    </span>
                    <span className={`instance-overview-state ${item.running ? "running" : "stopped"}`}>
                      {item.running ? "运行中" : "未运行"}
                    </span>
                  </div>
                  <div className="instance-overview-item-meta">
                    <span>{item.sessionCount} 条会话</span>
                    <span>{formatCwdLabel(item.user_data_dir)}</span>
                  </div>
                </button>
              ))}
            </div>
            <div className="instance-overview-actions">
              <button className="btn btn-ghost btn-xs" onClick={() => setShowCodexInstances(true)}>
                打开实例面板
              </button>
            </div>
          </div>

          <SessionGroupsList
            sessions={sessions}
            loading={loading}
            sessionGroups={sessionGroups}
            expandedGroups={expandedGroups}
            selectedFilepaths={selectedFilepaths}
            selectedSession={selectedSession}
            sourceFilter={sourceFilter}
            sessionSignalFilter={sessionSignalFilter}
            searchQuery={searchQuery}
            toolFilter={toolFilter}
            onClearFilters={() => {
              setSourceFilter("all");
              setSessionSignalFilter("all");
              setToolFilter("all");
              setSearchQuery("");
              setProblemAccountGroupFilter("all");
            }}
            onToggleGroupExpanded={toggleGroupExpanded}
            onToggleGroupSelected={toggleGroupSelected}
            onToggleSessionSelected={toggleSessionSelected}
            onLoadSession={loadSession}
            getSessionFlags={getSessionFlags}
            formatCwdLabel={formatCwdLabel}
            formatDate={formatDate}
            getSessionCwd={getSessionCwd}
          />
        </div>
      </div>

      {/* 详情主视口 */}
      <div className="session-content cyber-main">
        <SessionDetailPane
          selectedSession={selectedSession}
          visibleMessages={visibleMessages}
          messagesLoading={messagesLoading}
          messageViewMode={messageViewMode}
          relatedGeminiTranscripts={relatedGeminiTranscripts}
          selectedSessionToolOutputDir={selectedSessionToolOutputDir}
          collapsedToolKeys={collapsedToolKeys}
          onSetMessageViewMode={setMessageViewMode}
          onShowCodexInstances={() => setShowCodexInstances(true)}
          onJumpToRelatedGeminiTranscript={jumpToRelatedGeminiTranscript}
          onShowRelatedGeminiDialog={() => setShowRelatedGeminiDialog(true)}
          onCopyDir={handleCopyDir}
          onCopyCmd={handleCopyCmd}
          onCopyVisibleMessages={handleCopyVisibleMessages}
          onOpenToolOutputDir={handleOpenToolOutputDir}
          onLaunchTerminal={handleLaunchTerminal}
          onToggleToolMessageCollapsed={toggleToolMessageCollapsed}
          onCopyMessageBlock={handleCopyMessageBlock}
          onExpandMessage={setExpandedMessage}
          formatMessageTime={formatMessageTime}
          getSessionCwd={getSessionCwd}
          isToolRelatedMessage={isToolRelatedMessage}
        />
      </div>

      {showCodexInstances && (
        <div className="modal-overlay" onClick={() => setShowCodexInstances(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Codex 实例目录</h2>
              <button className="btn btn-icon" onClick={() => setShowCodexInstances(false)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="alert alert-info" style={{ marginBottom: 16 }}>
                默认实例 <code>~/.codex</code> 会自动生效，这里只管理额外实例目录。
              </div>

              {defaultCodexInstance && (
                <div className="session-instance-item" style={{ marginBottom: 16 }}>
                  <div>
                    <div className="session-instance-title">{defaultCodexInstance.name}</div>
                    <div className="session-instance-path">{defaultCodexInstance.user_data_dir}</div>
                    <div className="session-instance-meta">
                      <span>{defaultCodexInstance.running ? `运行中 PID ${defaultCodexInstance.last_pid}` : "未运行"}</span>
                      <span>{codexInstanceCards.find((item) => item.id === defaultCodexInstance.id)?.sessionCount ?? 0} 条会话</span>
                      <span>
                        {defaultCodexInstance.follow_local_account
                          ? `跟随当前本地账号 ${getCodexAccountLabel(currentCodexAccountId)}`
                          : `绑定账号 ${getCodexAccountLabel(defaultCodexInstance.bind_account_id)}`}
                      </span>
                      <span>{defaultCodexInstance.bind_provider_id ? `绑定 Provider ${defaultCodexInstance.bind_provider_id}` : "使用当前激活 Provider"}</span>
                      <span>{defaultCodexInstance.extra_args ? `参数 ${defaultCodexInstance.extra_args}` : "无额外参数"}</span>
                    </div>
                    <div className="session-instance-flags">
                      <span className={`instance-flag ${defaultCodexInstance.has_state_db ? "ok" : "bad"}`}>
                        state_5.sqlite
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_session_index ? "ok" : "bad"}`}>
                        session_index.jsonl
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_shared_skills ? "ok" : "bad"}`}>
                        skills
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_shared_rules ? "ok" : "bad"}`}>
                        rules
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_shared_vendor_imports_skills ? "ok" : "bad"}`}>
                        vendor_imports/skills
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_shared_agents_file ? "ok" : "bad"}`}>
                        AGENTS.md
                      </span>
                      {defaultCodexInstance.shared_strategy_version && (
                        <span className="instance-flag ok">
                          共享策略 {defaultCodexInstance.shared_strategy_version}
                        </span>
                      )}
                      {defaultCodexInstance.has_shared_conflicts && (
                        <span
                          className="instance-flag bad"
                          title={(defaultCodexInstance.shared_conflict_paths || []).join(", ") || "共享资源存在未托管冲突"}
                        >
                          共享冲突 {(defaultCodexInstance.shared_conflict_paths || []).length}
                        </span>
                      )}
                      <span className={`instance-flag ${getEffectiveCodexAccountId(defaultCodexInstance) ? "ok" : "bad"}`}>
                        {getEffectiveCodexAccountId(defaultCodexInstance)
                          ? `实际账号 ${getCodexAccountLabel(getEffectiveCodexAccountId(defaultCodexInstance))}`
                          : "当前无有效账号"}
                      </span>
                    </div>
                  </div>
                  <div className="session-instance-actions">
                    <span className="badge badge-success">默认</span>
                    <button className="btn btn-ghost btn-xs" onClick={() => handleCopyText(defaultCodexInstance.user_data_dir, "实例目录已复制")}>
                      复制路径
                    </button>
                    <button className="btn btn-ghost btn-xs" onClick={() => handleUpdateCodexInstanceSettings(defaultCodexInstance)}>
                      设置
                    </button>
                    <button
                      className="btn btn-ghost btn-xs"
                      onClick={() => handleCreateInstanceFloatingCard(defaultCodexInstance)}
                    >
                      创建浮窗
                    </button>
                    <button
                      className="btn btn-ghost btn-xs"
                      onClick={() => handleSyncCodexSharedResources(defaultCodexInstance)}
                      disabled={sharedSyncBusyInstanceId === defaultCodexInstance.id}
                    >
                      {sharedSyncBusyInstanceId === defaultCodexInstance.id ? "同步中..." : "重试共享同步"}
                    </button>
                    {defaultCodexInstance.has_shared_conflicts && (
                      <button
                        className="btn btn-ghost btn-xs"
                        onClick={() =>
                          handleCopyText(
                            (defaultCodexInstance.shared_conflict_paths || []).join("\n"),
                            `已复制 ${defaultCodexInstance.shared_conflict_paths?.length || 0} 项共享冲突路径`
                          )
                        }
                        disabled={(defaultCodexInstance.shared_conflict_paths || []).length === 0}
                      >
                        复制冲突清单
                      </button>
                    )}
                    {defaultCodexInstance.running ? (
                      <>
                        <button className="btn btn-ghost btn-xs" onClick={() => handleOpenCodexWindow(defaultCodexInstance.id)}>
                          打开
                        </button>
                        <button className="btn btn-danger-ghost btn-xs" onClick={() => handleStopCodexInstance(defaultCodexInstance.id)}>
                          停止
                        </button>
                      </>
                    ) : (
                      <button
                        className="btn btn-primary btn-xs"
                        onClick={() => handleStartCodexInstance(defaultCodexInstance.id)}
                        disabled={sharedSyncBusyInstanceId === defaultCodexInstance.id}
                      >
                        {sharedSyncBusyInstanceId === defaultCodexInstance.id ? "同步并启动中..." : "启动"}
                      </button>
                    )}
                  </div>
                </div>
              )}

              <div className="form-row">
                <label className="form-label">实例名称</label>
                <input
                  className="form-input"
                  value={codexInstanceName}
                  onChange={(e) => setCodexInstanceName(e.target.value)}
                  placeholder="例如：工作目录实例 / 沙盒实例"
                />
              </div>

              <div className="form-row">
                <label className="form-label">实例目录</label>
                <div style={{ display: "flex", gap: 8 }}>
                  <input
                    className="form-input font-mono"
                    value={codexInstanceDir}
                    onChange={(e) => setCodexInstanceDir(e.target.value)}
                    placeholder="选择或粘贴 Codex user data 目录"
                  />
                  <button type="button" className="btn btn-ghost" onClick={handlePickCodexDir}>
                    浏览
                  </button>
                </div>
              </div>

              <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 16 }}>
                <button className="btn btn-danger-ghost" style={{ marginRight: 8 }} onClick={handleCloseAllCodexInstances}>
                  全部关闭
                </button>
                <button className="btn btn-primary" onClick={handleAddCodexInstance} disabled={codexInstanceLoading}>
                  {codexInstanceLoading ? "添加中..." : "添加实例"}
                </button>
              </div>

              <div className="session-instance-list">
                {codexInstances.length === 0 ? (
                  <div className="empty-text">当前还没有额外 Codex 实例</div>
                ) : (
                  codexInstances.map((item) => (
                    <div key={item.id} className="session-instance-item">
                      <div>
                        <div className="session-instance-title">{item.name}</div>
                        <div className="session-instance-path">{item.user_data_dir}</div>
                        <div className="session-instance-meta">
                          <span>{item.running ? `运行中 PID ${item.last_pid}` : "未运行"}</span>
                          <span>{codexInstanceCards.find((card) => card.id === item.id)?.sessionCount ?? 0} 条会话</span>
                          <span>{`绑定账号 ${getCodexAccountLabel(item.bind_account_id)}`}</span>
                          <span>{item.bind_provider_id ? `绑定 Provider ${item.bind_provider_id}` : "使用当前激活 Provider"}</span>
                          <span>{item.extra_args ? `参数 ${item.extra_args}` : "无额外参数"}</span>
                        </div>
                        <div className="session-instance-flags">
                          <span className={`instance-flag ${item.has_state_db ? "ok" : "bad"}`}>
                            state_5.sqlite
                          </span>
                          <span className={`instance-flag ${item.has_session_index ? "ok" : "bad"}`}>
                            session_index.jsonl
                          </span>
                          <span className={`instance-flag ${item.has_shared_skills ? "ok" : "bad"}`}>
                            skills
                          </span>
                          <span className={`instance-flag ${item.has_shared_rules ? "ok" : "bad"}`}>
                            rules
                          </span>
                          <span className={`instance-flag ${item.has_shared_vendor_imports_skills ? "ok" : "bad"}`}>
                            vendor_imports/skills
                          </span>
                          <span className={`instance-flag ${item.has_shared_agents_file ? "ok" : "bad"}`}>
                            AGENTS.md
                          </span>
                          {item.shared_strategy_version && (
                            <span className="instance-flag ok">
                              共享策略 {item.shared_strategy_version}
                            </span>
                          )}
                          {item.has_shared_conflicts && (
                            <span
                              className="instance-flag bad"
                              title={(item.shared_conflict_paths || []).join(", ") || "共享资源存在未托管冲突"}
                            >
                              共享冲突 {(item.shared_conflict_paths || []).length}
                            </span>
                          )}
                          <span className={`instance-flag ${getEffectiveCodexAccountId(item) ? "ok" : "bad"}`}>
                            {getEffectiveCodexAccountId(item)
                              ? `实际账号 ${getCodexAccountLabel(getEffectiveCodexAccountId(item))}`
                              : "当前无有效账号"}
                          </span>
                        </div>
                      </div>
                      <div className="session-instance-actions">
                        <button className="btn btn-ghost btn-xs" onClick={() => handleCopyText(item.user_data_dir, "实例目录已复制")}>
                          复制路径
                        </button>
                        <button className="btn btn-ghost btn-xs" onClick={() => handleUpdateCodexInstanceSettings(item)}>
                          设置
                        </button>
                        <button
                          className="btn btn-ghost btn-xs"
                          onClick={() => handleCreateInstanceFloatingCard(item)}
                        >
                          创建浮窗
                        </button>
                        <button
                          className="btn btn-ghost btn-xs"
                          onClick={() => handleSyncCodexSharedResources(item)}
                          disabled={sharedSyncBusyInstanceId === item.id}
                        >
                          {sharedSyncBusyInstanceId === item.id ? "同步中..." : "重试共享同步"}
                        </button>
                        {item.has_shared_conflicts && (
                          <button
                            className="btn btn-ghost btn-xs"
                            onClick={() =>
                              handleCopyText(
                                (item.shared_conflict_paths || []).join("\n"),
                                `已复制 ${item.shared_conflict_paths?.length || 0} 项共享冲突路径`
                              )
                            }
                            disabled={(item.shared_conflict_paths || []).length === 0}
                          >
                            复制冲突清单
                          </button>
                        )}
                        {item.running ? (
                          <>
                            <button className="btn btn-ghost btn-xs" onClick={() => handleOpenCodexWindow(item.id)}>
                              打开
                            </button>
                            <button className="btn btn-danger-ghost btn-xs" onClick={() => handleStopCodexInstance(item.id)}>
                              停止
                            </button>
                          </>
                        ) : (
                          <button
                            className="btn btn-primary btn-xs"
                            onClick={() => handleStartCodexInstance(item.id)}
                            disabled={sharedSyncBusyInstanceId === item.id}
                          >
                            {sharedSyncBusyInstanceId === item.id ? "同步并启动中..." : "启动"}
                          </button>
                        )}
                        <button className="btn btn-danger-ghost btn-xs" onClick={() => handleDeleteCodexInstance(item.id)}>
                          删除
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      <ExpandedMessageModal
        message={expandedMessage}
        onClose={() => setExpandedMessage(null)}
        onOpenSource={handleOpenMessageSource}
        onCopyFull={handleCopyMessageBlock}
        formatMessageTime={formatMessageTime}
      />

      <RelatedGeminiTranscriptsDialog
        open={showRelatedGeminiDialog}
        selectedSession={selectedSession}
        relatedGeminiTranscripts={relatedGeminiTranscripts}
        filteredRelatedGeminiTranscripts={filteredRelatedGeminiTranscripts}
        relatedSearchQuery={relatedSearchQuery}
        relatedStatusFilter={relatedStatusFilter}
        onClose={() => setShowRelatedGeminiDialog(false)}
        onQueryChange={setRelatedSearchQuery}
        onStatusFilterChange={setRelatedStatusFilter}
        onSelectTranscript={async (item) => {
          await loadSession(item);
          setShowRelatedGeminiDialog(false);
        }}
        getSessionCwd={getSessionCwd}
        formatDate={formatDate}
      />

      {confirmDialog && (
        <div className="modal-overlay" onClick={() => !confirmDialogBusy && setConfirmDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>{confirmDialog.title}</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDialog(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>{confirmDialog.description}</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmDialog(null)} disabled={confirmDialogBusy}>取消</button>
                <button
                  className={confirmDialog.tone === "danger" ? "btn btn-danger" : "btn btn-primary"}
                  disabled={confirmDialogBusy}
                  onClick={async () => {
                    try {
                      setConfirmDialogBusy(true);
                      await confirmDialog.action();
                      setConfirmDialog(null);
                    } finally {
                      setConfirmDialogBusy(false);
                    }
                  }}
                >
                  {confirmDialogBusy ? "处理中..." : confirmDialog.confirmLabel}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {codexSettingsDialog && (
        <div className="modal-overlay" onClick={() => setCodexSettingsDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>设置 Codex 实例</h2>
              <button className="btn btn-icon" onClick={() => setCodexSettingsDialog(null)}>✕</button>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div className="form-row">
                <label className="form-label">额外启动参数</label>
                <input
                  className="form-input"
                  value={codexSettingsDialog.extraArgs}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, extraArgs: e.target.value })}
                />
              </div>
              <div className="form-row">
                <label className="form-label">绑定账号</label>
                <select
                  className="form-input"
                  value={codexSettingsDialog.bindAccountId}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, bindAccountId: e.target.value })}
                  disabled={codexSettingsDialog.instance.is_default && codexSettingsDialog.followLocalAccount}
                >
                  <option value="">未绑定账号</option>
                  {codexAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {(account.label?.trim() || account.email)}
                      {currentCodexAccountId === account.id ? " (当前本地)" : ""}
                    </option>
                  ))}
                </select>
                <div className="text-muted" style={{ fontSize: 12 }}>
                  {codexSettingsDialog.instance.is_default && codexSettingsDialog.followLocalAccount
                    ? `当前会跟随本地 Codex 账号：${getCodexAccountLabel(currentCodexAccountId)}`
                    : `当前选择：${getCodexAccountLabel(codexSettingsDialog.bindAccountId || null)}`}
                </div>
              </div>
              <div className="form-row">
                <label className="form-label">绑定 Provider</label>
                <select
                  className="form-input"
                  value={codexSettingsDialog.bindProviderId}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, bindProviderId: e.target.value })}
                >
                  <option value="">使用当前激活 Provider</option>
                  {codexProviders.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name}{provider.is_active ? " (当前激活)" : ""}
                    </option>
                  ))}
                </select>
              </div>
              {codexSettingsDialog.instance.is_default && (
                <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={codexSettingsDialog.followLocalAccount}
                    onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, followLocalAccount: e.target.checked })}
                  />
                  跟随当前本地 Codex 账号
                </label>
              )}
              {codexSettingsDialog.instance.is_default &&
                codexSettingsDialog.followLocalAccount &&
                !currentCodexAccountId && (
                  <div className="alert alert-warning" style={{ fontSize: 13 }}>
                    当前没有解析到本地 Codex 账号。若继续保持跟随模式，默认实例启动时不会有可注入的账号。
                  </div>
                )}
              {(!codexSettingsDialog.instance.is_default || !codexSettingsDialog.followLocalAccount) &&
                codexSettingsDialog.bindAccountId &&
                currentCodexAccountId &&
                codexSettingsDialog.bindAccountId !== currentCodexAccountId && (
                  <div className="alert alert-info" style={{ fontSize: 13 }}>
                    这个实例启动时会把当前本地账号从 {getCodexAccountLabel(currentCodexAccountId)} 切换为 {getCodexAccountLabel(codexSettingsDialog.bindAccountId)}。
                  </div>
                )}
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setCodexSettingsDialog(null)}>取消</button>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    try {
                      await invoke("update_codex_instance_settings", {
                        id: codexSettingsDialog.instance.id,
                        extraArgs: codexSettingsDialog.extraArgs,
                        bindAccountId: codexSettingsDialog.bindAccountId.trim() || null,
                        bindProviderId: codexSettingsDialog.bindProviderId.trim() || null,
                        followLocalAccount: codexSettingsDialog.instance.is_default
                          ? codexSettingsDialog.followLocalAccount
                          : undefined,
                      });
                      pushActionMessage({ text: "Codex 实例设置已更新", tone: "success" });
                      setCodexSettingsDialog(null);
                      await fetchAll();
                    } catch (e) {
                      pushActionMessage({ text: "更新 Codex 实例设置失败：" + String(e), tone: "error" });
                    }
                  }}
                >
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
