import type { ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import { SettingsDialogsContainer } from "./SettingsDialogsContainer";
import { SettingsPageSections } from "./SettingsPageSections";
import { SettingsWebdavPullDialog } from "./SettingsWebdavPullDialog";
import { useSettingsRuntimeData } from "./useSettingsRuntimeData";
import { useFloatingCards } from "./useFloatingCards";
import { useSettingsBackupAndUpdate } from "./useSettingsBackupAndUpdate";
import { useSettingsGemini } from "./useSettingsGemini";
import { useSettingsPageState } from "./useSettingsPageState";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const pageState = useSettingsPageState();
  const runtimeData = useSettingsRuntimeData();

  const floatingCardsActions = useFloatingCards({
    setFloatingCards: runtimeData.setFloatingCards,
    setFloatingCardMsg: pageState.setFloatingCardMsg,
  });

  const gemini = useSettingsGemini({
    ideAccounts: runtimeData.ideAccounts,
    currentGeminiAccountId: runtimeData.currentGeminiAccountId,
    geminiInstances: runtimeData.geminiInstances,
    defaultGeminiInstance: runtimeData.defaultGeminiInstance,
    geminiInstanceName: pageState.geminiInstanceName,
    geminiInstanceDir: pageState.geminiInstanceDir,
    geminiEditDialog: pageState.geminiEditDialog,
    setIdeAccounts: runtimeData.setIdeAccounts,
    setCurrentSnapshots: runtimeData.setCurrentSnapshots,
    setCurrentGeminiAccountId: runtimeData.setCurrentGeminiAccountId,
    setGeminiInstances: runtimeData.setGeminiInstances,
    setDefaultGeminiInstance: runtimeData.setDefaultGeminiInstance,
    setGeminiInstanceName: pageState.setGeminiInstanceName,
    setGeminiInstanceDir: pageState.setGeminiInstanceDir,
    setGeminiInstanceMsg: pageState.setGeminiInstanceMsg,
    setGeminiInstanceLoading: pageState.setGeminiInstanceLoading,
    setGeminiRefreshLoading: pageState.setGeminiRefreshLoading,
    setGeminiEditDialog: pageState.setGeminiEditDialog,
    setConfirmDeleteGeminiId: pageState.setConfirmDeleteGeminiId,
  });

  const backupAndUpdate = useSettingsBackupAndUpdate({
    updateSettings: runtimeData.updateSettings,
    updateRuntimeInfo: runtimeData.updateRuntimeInfo,
    availableUpdate: pageState.availableUpdate,
    webdavUrl: pageState.webdavUrl,
    webdavUser: pageState.webdavUser,
    webdavPass: pageState.webdavPass,
    setLoading: pageState.setLoading,
    setMessage: pageState.setMessage,
    setUpdateMsg: pageState.setUpdateMsg,
    setIsCheckingUpdate: pageState.setIsCheckingUpdate,
    setUpdateSettings: runtimeData.setUpdateSettings,
    setLinuxInstallBusyUrl: pageState.setLinuxInstallBusyUrl,
    setAvailableUpdate: pageState.setAvailableUpdate,
    setUpdateProgress: pageState.setUpdateProgress,
    setWebdavMsg: pageState.setWebdavMsg,
    setWebdavLoading: pageState.setWebdavLoading,
    setConfirmWebdavPull: pageState.setConfirmWebdavPull,
  });

  const handleLanguageChange = (e: ChangeEvent<HTMLSelectElement>) => {
    const lang = e.target.value;
    i18n.changeLanguage(lang);
    localStorage.setItem("ais_lang", lang);
  };

  return (
    <div>
      <SettingsPageSections
        pageTitle={t("settings.title")}
        pageSubtitle={t("settings.subtitle")}
        languageTitle={t("settings.language")}
        languageDescription={t("settings.language_desc")}
        autoUpdateTitle={t("settings.auto_update")}
        autoUpdateDescription={t("settings.auto_update_desc")}
        checkNowLabel={t("settings.check_now")}
        selectedLanguage={i18n.language.startsWith("zh") ? "zh" : "en"}
        onLanguageChange={handleLanguageChange}
        pageState={pageState}
        runtimeData={runtimeData}
        floatingCards={floatingCardsActions}
        gemini={gemini}
        backupAndUpdate={backupAndUpdate}
      />
      <SettingsDialogsContainer
        pageState={pageState}
        runtimeData={runtimeData}
        gemini={gemini}
      />
      <SettingsWebdavPullDialog
        open={pageState.confirmWebdavPull}
        webdavLoading={pageState.webdavLoading}
        onClose={() => pageState.setConfirmWebdavPull(false)}
        onConfirm={() => void backupAndUpdate.handleWebdavPull()}
      />
    </div>
  );
}
