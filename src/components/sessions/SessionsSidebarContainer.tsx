import { SessionsSidebar } from "./SessionsSidebar";
import type { ChatSession } from "./sessionTypes";
import {
  formatCwdLabel,
  formatDate,
  formatUptime,
  getSessionCwd,
  getSessionFlags,
  isProblemSession,
} from "./sessionUtils";
import type { SessionsActionsState } from "./useSessionsActions";
import type { SessionsDerivedState } from "./useSessionsDerivedState";
import type { SessionsPageState } from "./useSessionsPageState";

type SessionsSidebarContainerProps = {
  sessions: ChatSession[];
  selectedSession: ChatSession | null;
  pageState: SessionsPageState;
  derivedState: SessionsDerivedState;
  actions: SessionsActionsState;
};

export function SessionsSidebarContainer({
  sessions,
  selectedSession,
  pageState,
  derivedState,
  actions,
}: SessionsSidebarContainerProps) {
  const codexSessionCount = sessions.filter((item) => item.tool_type === "Codex").length;

  return (
    <SessionsSidebar
      loading={pageState.loading}
      zombies={pageState.zombies}
      codexInstanceCount={pageState.codexInstanceCount}
      selectedFilepaths={pageState.selectedFilepaths}
      selectedSessionsCount={derivedState.selectedSessions.length}
      selectedGroupsCount={derivedState.selectedGroupsCount}
      sessionsCount={sessions.length}
      sessions={sessions}
      sessionGroups={derivedState.sessionGroups}
      expandedGroups={pageState.expandedGroups}
      selectedSession={selectedSession}
      sourceFilter={pageState.sourceFilter}
      sessionSignalFilter={pageState.sessionSignalFilter}
      searchQuery={pageState.searchQuery}
      toolFilter={pageState.toolFilter}
      visibleSessionFilepaths={derivedState.visibleSessionFilepaths}
      visibleProblemFilepaths={derivedState.visibleProblemFilepaths}
      workspaceHistoryFilepaths={derivedState.workspaceHistoryFilepaths}
      noTranscriptFilepaths={derivedState.noTranscriptFilepaths}
      visibleProblemDirs={derivedState.visibleProblemDirs}
      visibleProblemResumeCommands={derivedState.visibleProblemResumeCommands}
      problemGroupCount={derivedState.problemGroupCount}
      problemAccountGroupFilter={pageState.problemAccountGroupFilter}
      problemAccountGroupOptions={derivedState.problemAccountGroupOptions}
      hasUngroupedProblemSessions={derivedState.hasUngroupedProblemSessions}
      toolFilterOptions={derivedState.toolFilterOptions}
      sessionOverview={derivedState.sessionOverview}
      currentSessionViewLabel={derivedState.currentSessionViewLabel}
      actionMessages={actions.actionMessages}
      codexInstanceCards={derivedState.codexInstanceCards}
      codexSessionCount={codexSessionCount}
      runningCodexInstances={derivedState.runningCodexInstances}
      onRepairCodexIndex={actions.handleRepairCodexIndex}
      onSyncCodexThreads={actions.handleSyncCodexThreads}
      onShowCodexInstances={() => pageState.setShowCodexInstances(true)}
      onMoveToTrash={actions.handleMoveToTrash}
      onRefresh={actions.fetchAll}
      onClearActionMessages={actions.clearActionMessages}
      onPushActionMessage={actions.pushActionMessage}
      onSearchQueryChange={pageState.setSearchQuery}
      onToolFilterChange={pageState.setToolFilter}
      onSourceFilterChange={pageState.setSourceFilter}
      onSessionSignalFilterChange={pageState.setSessionSignalFilter}
      onProblemAccountGroupFilterChange={pageState.setProblemAccountGroupFilter}
      onExpandVisibleGroups={() =>
        pageState.setExpandedGroups(derivedState.sessionGroups.map((group) => group.cwd))
      }
      onExpandProblemGroups={() =>
        pageState.setExpandedGroups(
          derivedState.sessionGroups
            .filter((group) => group.sessions.some((item) => isProblemSession(item)))
            .map((group) => group.cwd)
        )
      }
      onCollapseAllGroups={() => pageState.setExpandedGroups([])}
      onSelectVisibleSessions={() =>
        pageState.setSelectedFilepaths((prev) => [
          ...new Set([...prev, ...derivedState.visibleSessionFilepaths]),
        ])
      }
      onSelectProblemSessions={() =>
        pageState.setSelectedFilepaths((prev) => [
          ...new Set([...prev, ...derivedState.visibleProblemFilepaths]),
        ])
      }
      onSelectWorkspaceHistory={() =>
        pageState.setSelectedFilepaths((prev) => [
          ...new Set([...prev, ...derivedState.workspaceHistoryFilepaths]),
        ])
      }
      onSelectNoTranscript={() =>
        pageState.setSelectedFilepaths((prev) => [
          ...new Set([...prev, ...derivedState.noTranscriptFilepaths]),
        ])
      }
      onMoveProblemSessionsToTrash={() =>
        actions.handleMoveSpecificSessionsToTrash(
          derivedState.visibleProblemFilepaths,
          "问题会话"
        )
      }
      onCopyProblemDirs={() =>
        actions.handleCopyText(
          derivedState.visibleProblemDirs.join("\n"),
          `已复制 ${derivedState.visibleProblemDirs.length} 条问题会话目录`
        )
      }
      onCopyProblemCommands={() =>
        actions.handleCopyText(
          derivedState.visibleProblemResumeCommands.join("\n"),
          `已复制 ${derivedState.visibleProblemResumeCommands.length} 条问题会话恢复命令`
        )
      }
      onLaunchProblemSessions={() => actions.handleLaunchProblemSessionsInTerminal(3)}
      onClearFilters={() => {
        pageState.setSourceFilter("all");
        pageState.setSessionSignalFilter("all");
        pageState.setToolFilter("all");
        pageState.setSearchQuery("");
        pageState.setProblemAccountGroupFilter("all");
      }}
      onToggleAllSessionsSelected={actions.toggleAllSessionsSelected}
      onClearSelectedFilepaths={() => pageState.setSelectedFilepaths([])}
      onToggleGroupExpanded={actions.toggleGroupExpanded}
      onToggleGroupSelected={actions.toggleGroupSelected}
      onToggleSessionSelected={actions.toggleSessionSelected}
      onLoadSession={actions.loadSession}
      getSessionFlags={getSessionFlags}
      formatCwdLabel={formatCwdLabel}
      formatDate={formatDate}
      formatUptime={formatUptime}
      getSessionCwd={getSessionCwd}
    />
  );
}
