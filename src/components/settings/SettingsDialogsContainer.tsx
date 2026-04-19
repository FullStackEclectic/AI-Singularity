import { SettingsGeminiDialogs } from "./SettingsGeminiDialogs";
import type { SettingsGeminiState } from "./useSettingsGemini";
import type { SettingsPageState } from "./useSettingsPageState";
import type { SettingsRuntimeDataState } from "./useSettingsRuntimeData";

type SettingsDialogsContainerProps = {
  pageState: SettingsPageState;
  runtimeData: SettingsRuntimeDataState;
  gemini: SettingsGeminiState;
};

export function SettingsDialogsContainer({
  pageState,
  runtimeData,
  gemini,
}: SettingsDialogsContainerProps) {
  return (
    <SettingsGeminiDialogs
      geminiEditDialog={pageState.geminiEditDialog}
      confirmDeleteGeminiId={pageState.confirmDeleteGeminiId}
      geminiAccounts={gemini.geminiAccounts}
      currentGeminiAccountId={runtimeData.currentGeminiAccountId}
      editEffectiveGeminiAccountId={gemini.editEffectiveGeminiAccountId}
      editSelectedGeminiProjectId={gemini.editSelectedGeminiProjectId}
      editAccountProjectId={gemini.editAccountProjectId}
      getGeminiAccountLabel={gemini.getGeminiAccountLabel}
      onCloseEdit={() => pageState.setGeminiEditDialog(null)}
      onApplyGeminiEditPatch={gemini.applyGeminiEditPatch}
      onSaveGeminiEdit={() => void gemini.handleSaveGeminiEdit()}
      onCloseDelete={() => pageState.setConfirmDeleteGeminiId(null)}
      onConfirmDelete={(id) => void gemini.handleDeleteGeminiInstance(id)}
    />
  );
}
