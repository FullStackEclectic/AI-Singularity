import { useState, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { api } from "../../lib/api";
import type { AccountGroup, IdeAccount } from "../../types";
import type {
  ActionMessage,
  ChatMessage,
  ChatSession,
  CodexInstanceRecord,
  CodexSettingsDialogState,
  ConfirmDialogState,
  ProviderOption,
  SessionActionMessage,
  SessionGroup,
  ZombieProcess,
} from "./sessionTypes";
import { formatMessageTime, getGeminiToolOutputDir, getResumeCommand, getSessionCwd } from "./sessionUtils";

type UseSessionsActionsParams = {
  sessions: ChatSession[];
  selectedSession: ChatSession | null;
  selectedFilepaths: string[];
  codexInstanceName: string;
  codexInstanceDir: string;
  visibleMessages: ChatMessage[];
  relatedGeminiTranscripts: ChatSession[];
  visibleProblemSessions: ChatSession[];
  setSessions: Dispatch<SetStateAction<ChatSession[]>>;
  setZombies: Dispatch<SetStateAction<ZombieProcess[]>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setSelectedSession: Dispatch<SetStateAction<ChatSession | null>>;
  setMessages: Dispatch<SetStateAction<ChatMessage[]>>;
  setMessagesLoading: Dispatch<SetStateAction<boolean>>;
  setMessageViewMode: Dispatch<SetStateAction<"all" | "tool">>;
  setShowRelatedGeminiDialog: Dispatch<SetStateAction<boolean>>;
  setRelatedSearchQuery: Dispatch<SetStateAction<string>>;
  setRelatedStatusFilter: Dispatch<SetStateAction<"all" | "success" | "failed" | "none">>;
  setCollapsedToolKeys: Dispatch<SetStateAction<string[]>>;
  setExpandedGroups: Dispatch<SetStateAction<string[]>>;
  setSelectedFilepaths: Dispatch<SetStateAction<string[]>>;
  setCodexInstances: Dispatch<SetStateAction<CodexInstanceRecord[]>>;
  setDefaultCodexInstance: Dispatch<SetStateAction<CodexInstanceRecord | null>>;
  setCodexProviders: Dispatch<SetStateAction<ProviderOption[]>>;
  setIdeAccounts: Dispatch<SetStateAction<IdeAccount[]>>;
  setAccountGroups: Dispatch<SetStateAction<AccountGroup[]>>;
  setCurrentSnapshots: Dispatch<SetStateAction<any[]>>;
  setCodexInstanceName: Dispatch<SetStateAction<string>>;
  setCodexInstanceDir: Dispatch<SetStateAction<string>>;
};

export function useSessionsActions({
  sessions,
  selectedSession,
  selectedFilepaths,
  codexInstanceName,
  codexInstanceDir,
  visibleMessages,
  relatedGeminiTranscripts,
  visibleProblemSessions,
  setSessions,
  setZombies,
  setLoading,
  setSelectedSession,
  setMessages,
  setMessagesLoading,
  setMessageViewMode,
  setShowRelatedGeminiDialog,
  setRelatedSearchQuery,
  setRelatedStatusFilter,
  setCollapsedToolKeys,
  setExpandedGroups,
  setSelectedFilepaths,
  setCodexInstances,
  setDefaultCodexInstance,
  setCodexProviders,
  setIdeAccounts,
  setAccountGroups,
  setCurrentSnapshots,
  setCodexInstanceName,
  setCodexInstanceDir,
}: UseSessionsActionsParams) {
  const [actionMessages, setActionMessages] = useState<SessionActionMessage[]>([]);
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null);
  const [confirmDialogBusy, setConfirmDialogBusy] = useState(false);
  const [codexSettingsDialog, setCodexSettingsDialog] = useState<CodexSettingsDialogState | null>(null);
  const [sharedSyncBusyInstanceId, setSharedSyncBusyInstanceId] = useState<string | null>(null);
  const [codexInstanceLoading, setCodexInstanceLoading] = useState(false);

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
      const [sessData, zombieData, codexData, defaultCodex, ideAccountsList, groups, snapshots] =
        await Promise.all([
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
      setSelectedFilepaths((prev) =>
        prev.filter((filepath) => sessData.some((item) => item.filepath === filepath))
      );
      setZombies(zombieData);
      setCodexInstances(codexData);
      setDefaultCodexInstance(defaultCodex);
      setIdeAccounts(ideAccountsList);
      setAccountGroups(groups);
      setCurrentSnapshots(snapshots);
      setCodexProviders(
        providerData.filter((item) => {
          try {
            const targets = item.tool_targets
              ? (JSON.parse(item.tool_targets) as string[])
              : ["claude_code"];
            return targets.includes("codex");
          } catch {
            return false;
          }
        })
      );
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
      const msgData = await invoke<ChatMessage[]>("get_session_details", {
        filepath: session.filepath,
      });
      setMessages(msgData);
    } catch (e) {
      console.error("Failed to load session details:", e);
      setMessages([]);
    } finally {
      setMessagesLoading(false);
    }
  };

  const handleCopyText = (text: string, successText: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => pushActionMessage({ text: successText, tone: "success" }))
      .catch((e) => pushActionMessage({ text: "复制失败：" + String(e), tone: "error" }));
  };

  const handleCopyCmd = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    const cmd = getResumeCommand(session);
    const fullCmd = cwd ? `cd /d "${cwd}" && ${cmd}` : cmd;
    navigator.clipboard
      .writeText(fullCmd)
      .then(() =>
        pushActionMessage({ text: "恢复指令已复制到剪贴板", tone: "success" })
      );
  };

  const handleCopyDir = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    if (cwd) {
      navigator.clipboard
        .writeText(cwd)
        .then(() => pushActionMessage({ text: "目录路径已复制", tone: "success" }));
    } else {
      pushActionMessage({
        text: "当前会话没有可推断的工作目录，请改用复制恢复命令。",
        tone: "info",
      });
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
    const allSelected = group.sessions.every((session) =>
      selectedFilepaths.includes(session.filepath)
    );
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
          const result = await invoke<{ message: string }>("move_sessions_to_trash", {
            filepaths: selectedFilepaths,
          });
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
          text: `Codex 实例已添加，共享资源同步完成（发现 ${
            added.shared_conflict_paths?.length || 0
          } 项冲突并已跳过）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: "Codex 实例已添加，共享资源同步完成", tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({
        text: "添加 Codex 实例失败（共享资源同步或实例登记失败）：" + String(e),
        tone: "error",
      });
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
          text: `Codex 实例已启动，共享资源同步完成（发现 ${
            started.shared_conflict_paths?.length || 0
          } 项冲突并已跳过）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: "Codex 实例已启动，共享资源同步完成", tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({
        text: "启动 Codex 实例失败（共享资源同步或启动失败）：" + String(e),
        tone: "error",
      });
    } finally {
      setSharedSyncBusyInstanceId(null);
    }
  };

  const handleSyncCodexSharedResources = async (instance: CodexInstanceRecord) => {
    setSharedSyncBusyInstanceId(instance.id);
    pushActionMessage({ text: `正在重试共享资源同步：${instance.name}`, tone: "info" });
    try {
      const synced = await invoke<CodexInstanceRecord>(
        "sync_codex_instance_shared_resources",
        { id: instance.id }
      );
      if (synced.has_shared_conflicts) {
        pushActionMessage({
          text: `${instance.name} 共享资源同步完成（仍有 ${
            synced.shared_conflict_paths?.length || 0
          } 项冲突）`,
          tone: "info",
        });
      } else {
        pushActionMessage({ text: `${instance.name} 共享资源同步完成`, tone: "success" });
      }
      await fetchAll();
    } catch (e) {
      pushActionMessage({
        text: `${instance.name} 共享资源同步失败：${String(e)}`,
        tone: "error",
      });
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
      pushActionMessage({
        text: `${instance.name} 的浮动账号卡片已创建`,
        tone: "success",
      });
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

  const handleCopyInstancePath = (instance: CodexInstanceRecord) => {
    handleCopyText(instance.user_data_dir, "实例目录已复制");
  };

  const handleCopyConflictPaths = (instance: CodexInstanceRecord) => {
    handleCopyText(
      (instance.shared_conflict_paths || []).join("\n"),
      `已复制 ${instance.shared_conflict_paths?.length || 0} 项共享冲突路径`
    );
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
    const source = onlyTool
      ? visibleMessages.filter((message) =>
          message.role === "tool" ||
          message.content.includes("[工具调用]") ||
          message.content.includes("工具调用：") ||
          message.content.includes("Gemini logs.json 已记录") ||
          message.content.includes("当前会话的工具输出目录")
        )
      : visibleMessages;
    if (source.length === 0) {
      pushActionMessage({
        text: onlyTool ? "当前没有可复制的工具调用摘要" : "当前没有可复制的消息",
        tone: "info",
      });
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

  const jumpToRelatedGeminiTranscript = async () => {
    const target = relatedGeminiTranscripts[0];
    if (!target) {
      pushActionMessage({ text: "当前工作区还没有可跳转的 Gemini 转录", tone: "info" });
      return;
    }
    await loadSession(target);
    pushActionMessage({ text: "已跳转到同工作区的最新 Gemini 转录", tone: "success" });
  };

  const handleConfirmDialogConfirm = async () => {
    if (!confirmDialog) return;
    try {
      setConfirmDialogBusy(true);
      await confirmDialog.action();
      setConfirmDialog(null);
    } finally {
      setConfirmDialogBusy(false);
    }
  };

  const handleSaveCodexInstanceSettings = async () => {
    if (!codexSettingsDialog) return;
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
          const result = await invoke<{ message: string }>("move_sessions_to_trash", {
            filepaths,
          });
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
        pushActionMessage({
          text: `拉起会话失败：${session.title} · ${String(e)}`,
          tone: "error",
        });
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

  return {
    actionMessages,
    clearActionMessages,
    pushActionMessage,
    confirmDialog,
    setConfirmDialog,
    confirmDialogBusy,
    codexSettingsDialog,
    setCodexSettingsDialog,
    sharedSyncBusyInstanceId,
    codexInstanceLoading,
    fetchAll,
    loadSession,
    handleCopyText,
    handleCopyCmd,
    handleCopyDir,
    handleLaunchTerminal,
    toggleGroupExpanded,
    toggleSessionSelected,
    toggleAllSessionsSelected,
    toggleGroupSelected,
    handleMoveToTrash,
    handleRepairCodexIndex,
    handleSyncCodexThreads,
    handlePickCodexDir,
    handleAddCodexInstance,
    handleDeleteCodexInstance,
    handleUpdateCodexInstanceSettings,
    handleStartCodexInstance,
    handleSyncCodexSharedResources,
    handleStopCodexInstance,
    handleOpenCodexWindow,
    handleCreateInstanceFloatingCard,
    handleCloseAllCodexInstances,
    handleCopyInstancePath,
    handleCopyConflictPaths,
    handleCopyMessageBlock,
    handleCopyVisibleMessages,
    handleOpenToolOutputDir,
    handleOpenMessageSource,
    toggleToolMessageCollapsed,
    jumpToRelatedGeminiTranscript,
    handleConfirmDialogConfirm,
    handleSaveCodexInstanceSettings,
    handleMoveSpecificSessionsToTrash,
    handleLaunchProblemSessionsInTerminal,
  };
}

export type SessionsActionsState = ReturnType<typeof useSessionsActions>;
