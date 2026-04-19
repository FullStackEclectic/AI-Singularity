import { UnifiedAccountsDialogs } from "./UnifiedAccountsDialogs";
import type { UnifiedAccountsDialogsState } from "./useUnifiedAccountsDialogs";
import type { UnifiedAccountsQueriesState } from "./useUnifiedAccountsQueries";

type UnifiedAccountsDialogsContainerProps = {
  dialogs: UnifiedAccountsDialogsState;
  queries: UnifiedAccountsQueriesState;
};

export function UnifiedAccountsDialogsContainer({
  dialogs,
  queries,
}: UnifiedAccountsDialogsContainerProps) {
  return (
    <UnifiedAccountsDialogs
      showAddWizard={dialogs.showAddWizard}
      onCloseAddWizard={() => dialogs.setShowAddWizard(false)}
      onAddWizardSuccess={dialogs.handleAddWizardSuccess}
      confirmDialog={dialogs.confirmDialog}
      confirmDialogBusy={dialogs.confirmDialogBusy}
      setConfirmDialogBusy={dialogs.setConfirmDialogBusy}
      onCloseConfirmDialog={() => dialogs.setConfirmDialog(null)}
      geminiProjectDialog={dialogs.geminiProjectDialog}
      setGeminiProjectDialog={dialogs.setGeminiProjectDialog}
      geminiProjectBusy={dialogs.geminiProjectBusy}
      onCloseGeminiProjectDialog={() => dialogs.setGeminiProjectDialog(null)}
      onClearGeminiProject={dialogs.handleClearGeminiProject}
      onSaveGeminiProject={dialogs.handleSaveGeminiProject}
      codexApiKeyDialog={dialogs.codexApiKeyDialog}
      setCodexApiKeyDialog={dialogs.setCodexApiKeyDialog}
      codexApiKeyBusy={dialogs.codexApiKeyBusy}
      onCloseCodexApiKeyDialog={() => dialogs.setCodexApiKeyDialog(null)}
      onSaveCodexApiKey={dialogs.handleSaveCodexApiKey}
      ideLabelDialog={dialogs.ideLabelDialog}
      setIdeLabelDialog={dialogs.setIdeLabelDialog}
      ideLabelBusy={dialogs.ideLabelBusy}
      onCloseIdeLabelDialog={() => dialogs.setIdeLabelDialog(null)}
      onSaveIdeLabel={dialogs.handleSaveIdeLabel}
      batchIdeTagsDialog={dialogs.batchIdeTagsDialog}
      setBatchIdeTagsDialog={dialogs.setBatchIdeTagsDialog}
      batchIdeTagsBusy={dialogs.batchIdeTagsBusy}
      onCloseBatchIdeTagsDialog={() => dialogs.setBatchIdeTagsDialog(null)}
      onSaveBatchIdeTags={dialogs.handleSaveBatchIdeTags}
      accountGroupDialog={dialogs.accountGroupDialog}
      accountGroupBusy={dialogs.accountGroupBusy}
      accountGroups={queries.accountGroups}
      newGroupName={dialogs.newGroupName}
      renamingGroupId={dialogs.renamingGroupId}
      renamingGroupName={dialogs.renamingGroupName}
      onCloseAccountGroupDialog={() => dialogs.setAccountGroupDialog(null)}
      onNewGroupNameChange={dialogs.setNewGroupName}
      onCreateGroup={() => void dialogs.handleCreateAccountGroup()}
      onStartRenameGroup={dialogs.handleStartRenamingGroup}
      onRenamingGroupNameChange={dialogs.setRenamingGroupName}
      onSaveRenameGroup={(groupId) => void dialogs.handleSaveRenamingGroup(groupId)}
      onCancelRenameGroup={dialogs.handleCancelRenamingGroup}
      onDeleteAccountGroup={(group) => void dialogs.handleDeleteAccountGroup(group)}
      onAssignAccountsToGroup={(group) => void dialogs.handleAssignAccountsToGroup(group)}
      onRemoveAccountsFromGroup={(group) => void dialogs.handleRemoveAccountsFromGroup(group)}
    />
  );
}
