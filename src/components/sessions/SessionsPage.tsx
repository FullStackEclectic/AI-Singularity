import { useEffect, useState } from "react";
import type { CurrentAccountSnapshot } from "../../lib/api";
import { SessionsSidebarContainer } from "./SessionsSidebarContainer";
import { SessionsWorkspace } from "./SessionsWorkspace";
import type {
  ChatMessage,
  ChatSession,
  CodexInstanceRecord,
} from "./sessionTypes";
import { useSessionsActions } from "./useSessionsActions";
import { useSessionsDerivedState } from "./useSessionsDerivedState";
import { useSessionsPageState } from "./useSessionsPageState";
import type { AccountGroup, IdeAccount } from "../../types";
import "./SessionsPage.css";
import "./SessionsDialogs.css";

export default function SessionsPage() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [selectedSession, setSelectedSession] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [codexInstances, setCodexInstances] = useState<CodexInstanceRecord[]>([]);
  const [defaultCodexInstance, setDefaultCodexInstance] = useState<CodexInstanceRecord | null>(null);
  const [ideAccounts, setIdeAccounts] = useState<IdeAccount[]>([]);
  const [accountGroups, setAccountGroups] = useState<AccountGroup[]>([]);
  const [currentSnapshots, setCurrentSnapshots] = useState<CurrentAccountSnapshot[]>([]);

  const pageState = useSessionsPageState({
    sessions,
    selectedSession,
    messages,
    accountGroups,
    codexInstances,
  });

  const derivedState = useSessionsDerivedState({
    sessions,
    selectedFilepaths: pageState.selectedFilepaths,
    searchQuery: pageState.searchQuery,
    toolFilter: pageState.toolFilter,
    sourceFilter: pageState.sourceFilter,
    sessionSignalFilter: pageState.sessionSignalFilter,
    problemAccountGroupFilter: pageState.problemAccountGroupFilter,
    accountGroups,
    codexInstances,
    defaultCodexInstance,
    currentSnapshots,
    ideAccounts,
  });

  const actions = useSessionsActions({
    sessions,
    selectedSession,
    selectedFilepaths: pageState.selectedFilepaths,
    codexInstanceName: pageState.codexInstanceName,
    codexInstanceDir: pageState.codexInstanceDir,
    visibleMessages: pageState.visibleMessages,
    relatedGeminiTranscripts: pageState.relatedGeminiTranscripts,
    visibleProblemSessions: derivedState.visibleProblemSessions,
    setSessions,
    setZombies: pageState.setZombies,
    setLoading: pageState.setLoading,
    setSelectedSession,
    setMessages,
    setMessagesLoading: pageState.setMessagesLoading,
    setMessageViewMode: pageState.setMessageViewMode,
    setShowRelatedGeminiDialog: pageState.setShowRelatedGeminiDialog,
    setRelatedSearchQuery: pageState.setRelatedSearchQuery,
    setRelatedStatusFilter: pageState.setRelatedStatusFilter,
    setCollapsedToolKeys: pageState.setCollapsedToolKeys,
    setExpandedGroups: pageState.setExpandedGroups,
    setSelectedFilepaths: pageState.setSelectedFilepaths,
    setCodexInstances,
    setDefaultCodexInstance,
    setCodexProviders: pageState.setCodexProviders,
    setIdeAccounts,
    setAccountGroups,
    setCurrentSnapshots,
    setCodexInstanceName: pageState.setCodexInstanceName,
    setCodexInstanceDir: pageState.setCodexInstanceDir,
  });

  useEffect(() => {
    actions.fetchAll();
    const interval = setInterval(actions.fetchAll, 10000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="sessions-page cyberpunk-theme">
      <SessionsSidebarContainer
        sessions={sessions}
        selectedSession={selectedSession}
        pageState={pageState}
        derivedState={derivedState}
        actions={actions}
      />
      <SessionsWorkspace
        selectedSession={selectedSession}
        pageState={pageState}
        derivedState={derivedState}
        actions={actions}
      />
    </div>
  );
}
