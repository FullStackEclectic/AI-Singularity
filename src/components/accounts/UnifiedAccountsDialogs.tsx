import type { Dispatch, SetStateAction } from "react";
import type { AccountGroup } from "../../types";
import AddAccountWizard from "./AddAccountWizard";
import { UnifiedAccountsGroupDialog } from "./UnifiedAccountsGroupDialog";
import {
  BatchIdeTagsDialogModal,
  type BatchIdeTagsDialogState,
  CodexApiKeyDialogModal,
  type CodexApiKeyDialogState,
  ConfirmDialogModal,
  type ConfirmDialogState,
  GeminiProjectDialogModal,
  type GeminiProjectDialogState,
  IdeLabelDialogModal,
  type IdeLabelDialogState,
} from "./UnifiedAccountsModals";
import type { AccountGroupDialogState } from "./unifiedAccountsTypes";

export function UnifiedAccountsDialogs({
  showAddWizard,
  onCloseAddWizard,
  onAddWizardSuccess,
  confirmDialog,
  confirmDialogBusy,
  setConfirmDialogBusy,
  onCloseConfirmDialog,
  geminiProjectDialog,
  setGeminiProjectDialog,
  geminiProjectBusy,
  onCloseGeminiProjectDialog,
  onClearGeminiProject,
  onSaveGeminiProject,
  codexApiKeyDialog,
  setCodexApiKeyDialog,
  codexApiKeyBusy,
  onCloseCodexApiKeyDialog,
  onSaveCodexApiKey,
  ideLabelDialog,
  setIdeLabelDialog,
  ideLabelBusy,
  onCloseIdeLabelDialog,
  onSaveIdeLabel,
  batchIdeTagsDialog,
  setBatchIdeTagsDialog,
  batchIdeTagsBusy,
  onCloseBatchIdeTagsDialog,
  onSaveBatchIdeTags,
  accountGroupDialog,
  accountGroupBusy,
  accountGroups,
  newGroupName,
  renamingGroupId,
  renamingGroupName,
  onCloseAccountGroupDialog,
  onNewGroupNameChange,
  onCreateGroup,
  onStartRenameGroup,
  onRenamingGroupNameChange,
  onSaveRenameGroup,
  onCancelRenameGroup,
  onDeleteAccountGroup,
  onAssignAccountsToGroup,
  onRemoveAccountsFromGroup,
}: {
  showAddWizard: boolean;
  onCloseAddWizard: () => void;
  onAddWizardSuccess: () => void;
  confirmDialog: ConfirmDialogState | null;
  confirmDialogBusy: boolean;
  setConfirmDialogBusy: Dispatch<SetStateAction<boolean>>;
  onCloseConfirmDialog: () => void;
  geminiProjectDialog: GeminiProjectDialogState | null;
  setGeminiProjectDialog: Dispatch<SetStateAction<GeminiProjectDialogState | null>>;
  geminiProjectBusy: boolean;
  onCloseGeminiProjectDialog: () => void;
  onClearGeminiProject: () => Promise<void>;
  onSaveGeminiProject: () => Promise<void>;
  codexApiKeyDialog: CodexApiKeyDialogState | null;
  setCodexApiKeyDialog: Dispatch<SetStateAction<CodexApiKeyDialogState | null>>;
  codexApiKeyBusy: boolean;
  onCloseCodexApiKeyDialog: () => void;
  onSaveCodexApiKey: () => Promise<void>;
  ideLabelDialog: IdeLabelDialogState | null;
  setIdeLabelDialog: Dispatch<SetStateAction<IdeLabelDialogState | null>>;
  ideLabelBusy: boolean;
  onCloseIdeLabelDialog: () => void;
  onSaveIdeLabel: () => Promise<void>;
  batchIdeTagsDialog: BatchIdeTagsDialogState | null;
  setBatchIdeTagsDialog: Dispatch<SetStateAction<BatchIdeTagsDialogState | null>>;
  batchIdeTagsBusy: boolean;
  onCloseBatchIdeTagsDialog: () => void;
  onSaveBatchIdeTags: () => Promise<void>;
  accountGroupDialog: AccountGroupDialogState | null;
  accountGroupBusy: boolean;
  accountGroups: AccountGroup[];
  newGroupName: string;
  renamingGroupId: string | null;
  renamingGroupName: string;
  onCloseAccountGroupDialog: () => void;
  onNewGroupNameChange: (value: string) => void;
  onCreateGroup: () => void;
  onStartRenameGroup: (group: AccountGroup) => void;
  onRenamingGroupNameChange: (value: string) => void;
  onSaveRenameGroup: (groupId: string) => void;
  onCancelRenameGroup: () => void;
  onDeleteAccountGroup: (group: AccountGroup) => void;
  onAssignAccountsToGroup: (group: AccountGroup) => void;
  onRemoveAccountsFromGroup: (group: AccountGroup) => void;
}) {
  return (
    <>
      {showAddWizard && (
        <AddAccountWizard onClose={onCloseAddWizard} onSuccess={onAddWizardSuccess} />
      )}

      <ConfirmDialogModal
        dialog={confirmDialog}
        busy={confirmDialogBusy}
        setBusy={setConfirmDialogBusy}
        onClose={onCloseConfirmDialog}
      />

      <GeminiProjectDialogModal
        dialog={geminiProjectDialog}
        setDialog={setGeminiProjectDialog}
        busy={geminiProjectBusy}
        onClose={onCloseGeminiProjectDialog}
        onClear={onClearGeminiProject}
        onSave={onSaveGeminiProject}
      />

      <CodexApiKeyDialogModal
        dialog={codexApiKeyDialog}
        setDialog={setCodexApiKeyDialog}
        busy={codexApiKeyBusy}
        onClose={onCloseCodexApiKeyDialog}
        onSave={onSaveCodexApiKey}
      />

      <IdeLabelDialogModal
        dialog={ideLabelDialog}
        setDialog={setIdeLabelDialog}
        busy={ideLabelBusy}
        onClose={onCloseIdeLabelDialog}
        onSave={onSaveIdeLabel}
      />

      <BatchIdeTagsDialogModal
        dialog={batchIdeTagsDialog}
        setDialog={setBatchIdeTagsDialog}
        busy={batchIdeTagsBusy}
        onClose={onCloseBatchIdeTagsDialog}
        onSave={onSaveBatchIdeTags}
      />

      <UnifiedAccountsGroupDialog
        accountGroupDialog={accountGroupDialog}
        accountGroupBusy={accountGroupBusy}
        accountGroups={accountGroups}
        newGroupName={newGroupName}
        renamingGroupId={renamingGroupId}
        renamingGroupName={renamingGroupName}
        onClose={onCloseAccountGroupDialog}
        onNewGroupNameChange={onNewGroupNameChange}
        onCreateGroup={onCreateGroup}
        onStartRename={onStartRenameGroup}
        onRenamingGroupNameChange={onRenamingGroupNameChange}
        onSaveRename={onSaveRenameGroup}
        onCancelRename={onCancelRenameGroup}
        onDeleteGroup={onDeleteAccountGroup}
        onAssignToGroup={onAssignAccountsToGroup}
        onRemoveFromGroup={onRemoveAccountsFromGroup}
      />
    </>
  );
}
