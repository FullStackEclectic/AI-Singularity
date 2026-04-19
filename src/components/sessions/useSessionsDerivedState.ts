import { useMemo } from "react";
import type { CurrentAccountSnapshot } from "../../lib/api";
import type { AccountGroup, IdeAccount } from "../../types";
import type {
  ChatSession,
  CodexInstanceCardRecord,
  CodexInstanceRecord,
  SessionGroup,
} from "./sessionTypes";
import {
  buildResumeCommandWithCwd,
  getSessionCwd,
  isNoTranscriptSession,
  isProblemSession,
  isWorkspaceHistorySession,
} from "./sessionUtils";

type SessionSignalFilter = "all" | "tool" | "log" | "failed_tool";
type SourceFilter = "all" | "transcript" | "workspace_history" | "no_transcript";

type UseSessionsDerivedStateParams = {
  sessions: ChatSession[];
  selectedFilepaths: string[];
  searchQuery: string;
  toolFilter: string;
  sourceFilter: SourceFilter;
  sessionSignalFilter: SessionSignalFilter;
  problemAccountGroupFilter: string;
  accountGroups: AccountGroup[];
  codexInstances: CodexInstanceRecord[];
  defaultCodexInstance: CodexInstanceRecord | null;
  currentSnapshots: CurrentAccountSnapshot[];
  ideAccounts: IdeAccount[];
};

function matchesSessionFiltersWithoutProblemGroup(
  session: ChatSession,
  normalizedQuery: string,
  toolFilter: string,
  sourceFilter: SourceFilter,
  sessionSignalFilter: SessionSignalFilter
) {
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
}

export type SessionOverview = {
  total: number;
  visible: number;
  workspaceHistory: number;
  noTranscript: number;
  visibleProblem: number;
};

export function useSessionsDerivedState({
  sessions,
  selectedFilepaths,
  searchQuery,
  toolFilter,
  sourceFilter,
  sessionSignalFilter,
  problemAccountGroupFilter,
  accountGroups,
  codexInstances,
  defaultCodexInstance,
  currentSnapshots,
  ideAccounts,
}: UseSessionsDerivedStateParams) {
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

  const sessionGroups = useMemo<SessionGroup[]>(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();
    const grouped = new Map<string, ChatSession[]>();

    for (const session of sessions) {
      if (
        !matchesSessionFiltersWithoutProblemGroup(
          session,
          normalizedQuery,
          toolFilter,
          sourceFilter,
          sessionSignalFilter
        )
      ) {
        continue;
      }
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
        const label = cwd.startsWith("[") ? cwd : parts[parts.length - 1] || cwd;
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
    () =>
      [
        ...new Set(
          visibleProblemSessions
            .map((item) => getSessionCwd(item))
            .filter((item) => !!item)
        ),
      ],
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
      if (
        !matchesSessionFiltersWithoutProblemGroup(
          session,
          normalizedQuery,
          toolFilter,
          sourceFilter,
          sessionSignalFilter
        )
      ) {
        continue;
      }
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
    const workspaceHistoryCount = sessions.filter(
      (item) => item.source_kind === "workspace_history"
    ).length;
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
    return (
      accountGroups.find((group) => group.id === problemAccountGroupFilter)?.name || "未知分组"
    );
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
  }, [
    sessionSignalFilter,
    sourceFilter,
    searchQuery,
    toolFilter,
    problemGroupFilterLabel,
  ]);

  const selectedGroupsCount = useMemo(
    () =>
      sessionGroups.filter((group) =>
        group.sessions.some((item) => selectedFilepaths.includes(item.filepath))
      ).length,
    [sessionGroups, selectedFilepaths]
  );

  const problemGroupCount = useMemo(
    () =>
      sessionGroups.filter((group) => group.sessions.some((item) => isProblemSession(item)))
        .length,
    [sessionGroups]
  );

  const codexInstanceCards = useMemo<CodexInstanceCardRecord[]>(() => {
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
    () =>
      currentSnapshots.find((item) => item.platform === "codex")?.account_id ?? null,
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

  return {
    sessionGroups,
    selectedSessions,
    visibleSessions,
    visibleSessionFilepaths,
    visibleProblemSessions,
    visibleProblemFilepaths,
    visibleProblemDirs,
    visibleProblemResumeCommands,
    workspaceHistoryFilepaths,
    noTranscriptFilepaths,
    problemAccountGroupOptions,
    hasUngroupedProblemSessions,
    toolFilterOptions,
    sessionOverview,
    currentSessionViewLabel,
    selectedGroupsCount,
    problemGroupCount,
    codexInstanceCards,
    runningCodexInstances,
    codexAccounts,
    currentCodexAccountId,
    getCodexAccountLabel,
    getEffectiveCodexAccountId,
  };
}

export type SessionsDerivedState = ReturnType<typeof useSessionsDerivedState>;
