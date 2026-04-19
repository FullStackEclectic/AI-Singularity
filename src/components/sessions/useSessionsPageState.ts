import { useEffect, useMemo, useState } from "react";
import type { AccountGroup } from "../../types";
import type {
  ChatMessage,
  ChatSession,
  CodexInstanceRecord,
  ProviderOption,
  ZombieProcess,
} from "./sessionTypes";
import {
  getGeminiToolOutputDir,
  getSessionCwd,
  isToolRelatedMessage,
} from "./sessionUtils";

type UseSessionsPageStateParams = {
  sessions: ChatSession[];
  selectedSession: ChatSession | null;
  messages: ChatMessage[];
  accountGroups: AccountGroup[];
  codexInstances: CodexInstanceRecord[];
};

export function useSessionsPageState({
  sessions,
  selectedSession,
  messages,
  accountGroups,
  codexInstances,
}: UseSessionsPageStateParams) {
  const [zombies, setZombies] = useState<ZombieProcess[]>([]);
  const [loading, setLoading] = useState(false);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [messageViewMode, setMessageViewMode] = useState<"all" | "tool">("all");
  const [sessionSignalFilter, setSessionSignalFilter] = useState<
    "all" | "tool" | "log" | "failed_tool"
  >("all");
  const [expandedMessage, setExpandedMessage] = useState<ChatMessage | null>(null);
  const [showRelatedGeminiDialog, setShowRelatedGeminiDialog] = useState(false);
  const [relatedSearchQuery, setRelatedSearchQuery] = useState("");
  const [relatedStatusFilter, setRelatedStatusFilter] = useState<
    "all" | "success" | "failed" | "none"
  >("all");
  const [collapsedToolKeys, setCollapsedToolKeys] = useState<string[]>([]);
  const [expandedGroups, setExpandedGroups] = useState<string[]>([]);
  const [selectedFilepaths, setSelectedFilepaths] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [toolFilter, setToolFilter] = useState<string>("all");
  const [sourceFilter, setSourceFilter] = useState<
    "all" | "transcript" | "workspace_history" | "no_transcript"
  >("all");
  const [problemAccountGroupFilter, setProblemAccountGroupFilter] =
    useState<string>("all");
  const [showCodexInstances, setShowCodexInstances] = useState(false);
  const [codexInstanceName, setCodexInstanceName] = useState("");
  const [codexInstanceDir, setCodexInstanceDir] = useState("");
  const [codexProviders, setCodexProviders] = useState<ProviderOption[]>([]);

  const relatedGeminiTranscripts = useMemo(() => {
    if (!selectedSession || selectedSession.tool_type !== "GeminiCLI") return [];
    const cwd = getSessionCwd(selectedSession);
    if (!cwd) return [];
    return sessions
      .filter(
        (item) =>
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

  useEffect(() => {
    if (problemAccountGroupFilter === "all" || problemAccountGroupFilter === "__ungrouped__") {
      return;
    }
    if (!accountGroups.some((group) => group.id === problemAccountGroupFilter)) {
      setProblemAccountGroupFilter("all");
    }
  }, [accountGroups, problemAccountGroupFilter]);

  const codexInstanceCount = codexInstances.length + 1;

  return {
    zombies,
    setZombies,
    loading,
    setLoading,
    messagesLoading,
    setMessagesLoading,
    messageViewMode,
    setMessageViewMode,
    sessionSignalFilter,
    setSessionSignalFilter,
    expandedMessage,
    setExpandedMessage,
    showRelatedGeminiDialog,
    setShowRelatedGeminiDialog,
    relatedSearchQuery,
    setRelatedSearchQuery,
    relatedStatusFilter,
    setRelatedStatusFilter,
    collapsedToolKeys,
    setCollapsedToolKeys,
    expandedGroups,
    setExpandedGroups,
    selectedFilepaths,
    setSelectedFilepaths,
    searchQuery,
    setSearchQuery,
    toolFilter,
    setToolFilter,
    sourceFilter,
    setSourceFilter,
    problemAccountGroupFilter,
    setProblemAccountGroupFilter,
    showCodexInstances,
    setShowCodexInstances,
    codexInstanceName,
    setCodexInstanceName,
    codexInstanceDir,
    setCodexInstanceDir,
    codexProviders,
    setCodexProviders,
    relatedGeminiTranscripts,
    filteredRelatedGeminiTranscripts,
    visibleMessages,
    selectedSessionToolOutputDir,
    codexInstanceCount,
  };
}

export type SessionsPageState = ReturnType<typeof useSessionsPageState>;
