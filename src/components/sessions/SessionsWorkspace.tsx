import type { ChatSession } from "./sessionTypes";
import { ExpandedMessageModal } from "./ExpandedMessageModal";
import { RelatedGeminiTranscriptsDialog } from "./RelatedGeminiTranscriptsDialog";
import { SessionDetailPane } from "./SessionDetailPane";
import "./SessionsWorkspace.css";
import {
  CodexInstanceSettingsDialog,
  CodexInstancesModal,
  SessionsConfirmDialog,
} from "./SessionsDialogs";
import {
  formatDate,
  formatMessageTime,
  getSessionCwd,
  isToolRelatedMessage,
} from "./sessionUtils";
import type { SessionsActionsState } from "./useSessionsActions";
import type { SessionsDerivedState } from "./useSessionsDerivedState";
import type { SessionsPageState } from "./useSessionsPageState";

type SessionsWorkspaceProps = {
  selectedSession: ChatSession | null;
  pageState: SessionsPageState;
  derivedState: SessionsDerivedState;
  actions: SessionsActionsState;
};

export function SessionsWorkspace({
  selectedSession,
  pageState,
  derivedState,
  actions,
}: SessionsWorkspaceProps) {
  return (
    <>
      <div className="session-content cyber-main">
        <SessionDetailPane
          selectedSession={selectedSession}
          visibleMessages={pageState.visibleMessages}
          messagesLoading={pageState.messagesLoading}
          messageViewMode={pageState.messageViewMode}
          relatedGeminiTranscripts={pageState.relatedGeminiTranscripts}
          selectedSessionToolOutputDir={pageState.selectedSessionToolOutputDir}
          collapsedToolKeys={pageState.collapsedToolKeys}
          onSetMessageViewMode={pageState.setMessageViewMode}
          onShowCodexInstances={() => pageState.setShowCodexInstances(true)}
          onJumpToRelatedGeminiTranscript={actions.jumpToRelatedGeminiTranscript}
          onShowRelatedGeminiDialog={() => pageState.setShowRelatedGeminiDialog(true)}
          onCopyDir={actions.handleCopyDir}
          onCopyCmd={actions.handleCopyCmd}
          onCopyVisibleMessages={actions.handleCopyVisibleMessages}
          onOpenToolOutputDir={actions.handleOpenToolOutputDir}
          onLaunchTerminal={actions.handleLaunchTerminal}
          onToggleToolMessageCollapsed={actions.toggleToolMessageCollapsed}
          onCopyMessageBlock={actions.handleCopyMessageBlock}
          onExpandMessage={pageState.setExpandedMessage}
          formatMessageTime={formatMessageTime}
          getSessionCwd={getSessionCwd}
          isToolRelatedMessage={isToolRelatedMessage}
        />
      </div>

      <CodexInstancesModal
        open={pageState.showCodexInstances}
        codexInstanceCards={derivedState.codexInstanceCards}
        codexInstanceName={pageState.codexInstanceName}
        codexInstanceDir={pageState.codexInstanceDir}
        codexInstanceLoading={actions.codexInstanceLoading}
        sharedSyncBusyInstanceId={actions.sharedSyncBusyInstanceId}
        onClose={() => pageState.setShowCodexInstances(false)}
        onInstanceNameChange={pageState.setCodexInstanceName}
        onInstanceDirChange={pageState.setCodexInstanceDir}
        onPickCodexDir={actions.handlePickCodexDir}
        onAddCodexInstance={actions.handleAddCodexInstance}
        onCloseAllInstances={actions.handleCloseAllCodexInstances}
        getCodexAccountLabel={derivedState.getCodexAccountLabel}
        getEffectiveCodexAccountId={derivedState.getEffectiveCodexAccountId}
        onCopyPath={actions.handleCopyInstancePath}
        onOpenSettings={actions.handleUpdateCodexInstanceSettings}
        onCreateFloatingCard={actions.handleCreateInstanceFloatingCard}
        onSyncSharedResources={actions.handleSyncCodexSharedResources}
        onCopyConflictPaths={actions.handleCopyConflictPaths}
        onOpenWindow={actions.handleOpenCodexWindow}
        onStop={actions.handleStopCodexInstance}
        onStart={actions.handleStartCodexInstance}
        onDelete={actions.handleDeleteCodexInstance}
      />

      <ExpandedMessageModal
        message={pageState.expandedMessage}
        onClose={() => pageState.setExpandedMessage(null)}
        onOpenSource={actions.handleOpenMessageSource}
        onCopyFull={actions.handleCopyMessageBlock}
        formatMessageTime={formatMessageTime}
      />

      <RelatedGeminiTranscriptsDialog
        open={pageState.showRelatedGeminiDialog}
        selectedSession={selectedSession}
        relatedGeminiTranscripts={pageState.relatedGeminiTranscripts}
        filteredRelatedGeminiTranscripts={pageState.filteredRelatedGeminiTranscripts}
        relatedSearchQuery={pageState.relatedSearchQuery}
        relatedStatusFilter={pageState.relatedStatusFilter}
        onClose={() => pageState.setShowRelatedGeminiDialog(false)}
        onQueryChange={pageState.setRelatedSearchQuery}
        onStatusFilterChange={pageState.setRelatedStatusFilter}
        onSelectTranscript={async (item) => {
          await actions.loadSession(item);
          pageState.setShowRelatedGeminiDialog(false);
        }}
        getSessionCwd={getSessionCwd}
        formatDate={formatDate}
      />

      <SessionsConfirmDialog
        dialog={actions.confirmDialog}
        busy={actions.confirmDialogBusy}
        onClose={() => actions.setConfirmDialog(null)}
        onConfirm={actions.handleConfirmDialogConfirm}
      />

      <CodexInstanceSettingsDialog
        dialog={actions.codexSettingsDialog}
        codexAccounts={derivedState.codexAccounts}
        codexProviders={pageState.codexProviders}
        currentCodexAccountId={derivedState.currentCodexAccountId}
        onClose={() => actions.setCodexSettingsDialog(null)}
        onChange={actions.setCodexSettingsDialog}
        onSave={actions.handleSaveCodexInstanceSettings}
        getCodexAccountLabel={derivedState.getCodexAccountLabel}
      />
    </>
  );
}
