import type { ChangeEvent } from "react";
import { api } from "../../lib/api";
import { SettingsBackupAndUpdateSection } from "./SettingsBackupAndUpdateSection";
import { SettingsGeminiSection } from "./SettingsGeminiSection";
import { SettingsRuntimeSection } from "./SettingsRuntimeSection";
import type { FloatingCardsState } from "./useFloatingCards";
import type { SettingsBackupAndUpdateState } from "./useSettingsBackupAndUpdate";
import type { SettingsGeminiState } from "./useSettingsGemini";
import type { SettingsPageState } from "./useSettingsPageState";
import type { SettingsRuntimeDataState } from "./useSettingsRuntimeData";

type SettingsPageSectionsProps = {
  pageTitle: string;
  pageSubtitle: string;
  languageTitle: string;
  languageDescription: string;
  autoUpdateTitle: string;
  autoUpdateDescription: string;
  checkNowLabel: string;
  selectedLanguage: string;
  onLanguageChange: (event: ChangeEvent<HTMLSelectElement>) => void;
  pageState: SettingsPageState;
  runtimeData: SettingsRuntimeDataState;
  floatingCards: FloatingCardsState;
  gemini: SettingsGeminiState;
  backupAndUpdate: SettingsBackupAndUpdateState;
};

export function SettingsPageSections({
  pageTitle,
  pageSubtitle,
  languageTitle,
  languageDescription,
  autoUpdateTitle,
  autoUpdateDescription,
  checkNowLabel,
  selectedLanguage,
  onLanguageChange,
  pageState,
  runtimeData,
  floatingCards,
  gemini,
  backupAndUpdate,
}: SettingsPageSectionsProps) {
  return (
    <>
      <div className="page-header">
        <div>
          <h1 className="page-title">{pageTitle}</h1>
          <p className="page-subtitle">{pageSubtitle}</p>
        </div>
      </div>
      <div className="settings-section" style={{ padding: "var(--space-6)" }}>
        <SettingsRuntimeSection
          languageTitle={languageTitle}
          languageDescription={languageDescription}
          selectedLanguage={selectedLanguage}
          onLanguageChange={onLanguageChange}
          runtimeLoading={runtimeData.runtimeLoading}
          skillStorage={runtimeData.skillStorage}
          oauthEnvStatus={runtimeData.oauthEnvStatus}
          websocketStatus={runtimeData.websocketStatus}
          webReportStatus={runtimeData.webReportStatus}
          currentSnapshots={runtimeData.currentSnapshots}
          floatingCards={runtimeData.floatingCards}
          floatingCardMsg={pageState.floatingCardMsg}
          onCreateGlobalFloatingCard={() => void floatingCards.handleCreateGlobalFloatingCard()}
          onToggleFloatingCardVisible={(card) =>
            void floatingCards.handleToggleFloatingCardVisible(card)
          }
          onToggleFloatingCardTop={(card) =>
            void floatingCards.handleToggleFloatingCardTop(card)
          }
          onDeleteFloatingCard={(card) => void floatingCards.handleDeleteFloatingCard(card)}
        />

        <SettingsGeminiSection
          allVisibleGeminiInstances={gemini.allVisibleGeminiInstances}
          geminiUninitializedCount={gemini.geminiUninitializedCount}
          geminiConflictCount={gemini.geminiConflictCount}
          geminiProjectOverrideCount={gemini.geminiProjectOverrideCount}
          geminiUnboundCount={gemini.geminiUnboundCount}
          geminiRefreshLoading={pageState.geminiRefreshLoading}
          defaultGeminiInstance={runtimeData.defaultGeminiInstance}
          currentGeminiAccountId={runtimeData.currentGeminiAccountId}
          geminiInstanceName={pageState.geminiInstanceName}
          geminiInstanceDir={pageState.geminiInstanceDir}
          geminiInstanceMsg={pageState.geminiInstanceMsg}
          geminiInstanceLoading={pageState.geminiInstanceLoading}
          geminiInstances={runtimeData.geminiInstances}
          sortedGeminiInstances={gemini.sortedGeminiInstances}
          getGeminiAccountLabel={gemini.getGeminiAccountLabel}
          getEffectiveGeminiAccountId={gemini.getEffectiveGeminiAccountId}
          getEffectiveGeminiProjectId={gemini.getEffectiveGeminiProjectId}
          isCurrentLocalGeminiAccount={gemini.isCurrentLocalGeminiAccount}
          formatGeminiLaunchTime={gemini.formatGeminiLaunchTime}
          getGeminiInstanceWarnings={gemini.getGeminiInstanceWarnings}
          getGeminiAccountProjectLabel={gemini.getGeminiAccountProjectLabel}
          onRefreshGeminiRuntime={gemini.handleRefreshGeminiRuntime}
          onInstanceNameChange={pageState.setGeminiInstanceName}
          onInstanceDirChange={pageState.setGeminiInstanceDir}
          onPickGeminiDir={() => void gemini.handlePickGeminiDir()}
          onAddGeminiInstance={() => void gemini.handleAddGeminiInstance()}
          onQuickUpdateGeminiInstance={(instance, patch, successMessage) =>
            void gemini.handleQuickUpdateGeminiInstance(instance, patch, successMessage)
          }
          onOpenSettings={(instance) => void gemini.handleUpdateGeminiInstance(instance)}
          onCopyGeminiLaunchCommand={(id) => void gemini.handleCopyGeminiLaunchCommand(id)}
          onLaunchGeminiInstance={(id) => void gemini.handleLaunchGeminiInstance(id)}
          onConfirmDeleteGeminiInstance={pageState.setConfirmDeleteGeminiId}
        />

        <SettingsBackupAndUpdateSection
          autoUpdateTitle={autoUpdateTitle}
          autoUpdateDescription={autoUpdateDescription}
          checkNowLabel={checkNowLabel}
          loading={pageState.loading}
          message={pageState.message}
          webdavUrl={pageState.webdavUrl}
          webdavUser={pageState.webdavUser}
          webdavPass={pageState.webdavPass}
          webdavMsg={pageState.webdavMsg}
          webdavLoading={pageState.webdavLoading}
          updateSettings={runtimeData.updateSettings}
          updateRuntimeInfo={runtimeData.updateRuntimeInfo}
          linuxReleaseInfo={runtimeData.linuxReleaseInfo}
          linuxInstallBusyUrl={pageState.linuxInstallBusyUrl}
          availableUpdate={pageState.availableUpdate}
          isCheckingUpdate={pageState.isCheckingUpdate}
          updateMsg={pageState.updateMsg}
          updateProgress={pageState.updateProgress}
          selectedReminderStrategy={backupAndUpdate.selectedReminderStrategy}
          onExport={() => void backupAndUpdate.handleExport()}
          onImport={(event) => void backupAndUpdate.handleImport(event)}
          onWebdavUrlChange={pageState.setWebdavUrl}
          onWebdavUserChange={pageState.setWebdavUser}
          onWebdavPassChange={pageState.setWebdavPass}
          onTestWebdav={() => void backupAndUpdate.handleWebdavTest()}
          onPushWebdav={() => void backupAndUpdate.handleWebdavPush()}
          onOpenWebdavPullConfirm={() => pageState.setConfirmWebdavPull(true)}
          onUpdateSettingChange={(patch, successMessage) =>
            void backupAndUpdate.handleUpdateSettingChange(patch, successMessage)
          }
          onSkipFoundVersion={() => void backupAndUpdate.handleSkipFoundVersion()}
          onClearSkipVersion={() => void backupAndUpdate.handleClearSkipVersion()}
          onOpenAssetUrl={(url) => void api.update.openAssetUrl(url)}
          onInstallLinuxAsset={(url, kind, version) =>
            void backupAndUpdate.handleInstallLinuxAsset(url, kind, version)
          }
          onCheckUpdate={() => void backupAndUpdate.handleCheckUpdate()}
          onInstallUpdate={() => void backupAndUpdate.handleInstallUpdate()}
          onCollapseUpdateDetails={() => pageState.setAvailableUpdate(null)}
        />
      </div>
    </>
  );
}
