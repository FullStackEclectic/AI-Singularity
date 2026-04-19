import { useState } from "react";
import { Update as TauriUpdate } from "@tauri-apps/plugin-updater";
import type {
  GeminiEditDialogState,
  UpdateProgressState,
} from "./settingsTypes";

export function useSettingsPageState() {
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [linuxInstallBusyUrl, setLinuxInstallBusyUrl] = useState<string | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<TauriUpdate | null>(null);
  const [floatingCardMsg, setFloatingCardMsg] = useState("");
  const [updateProgress, setUpdateProgress] = useState<UpdateProgressState>({
    phase: "idle",
    downloaded: 0,
    total: 0,
  });
  const [webdavUrl, setWebdavUrl] = useState(localStorage.getItem("webdav_url") || "");
  const [webdavUser, setWebdavUser] = useState(localStorage.getItem("webdav_user") || "");
  const [webdavPass, setWebdavPass] = useState(localStorage.getItem("webdav_pass") || "");
  const [webdavMsg, setWebdavMsg] = useState("");
  const [webdavLoading, setWebdavLoading] = useState(false);
  const [geminiInstanceName, setGeminiInstanceName] = useState("");
  const [geminiInstanceDir, setGeminiInstanceDir] = useState("");
  const [geminiInstanceMsg, setGeminiInstanceMsg] = useState("");
  const [geminiInstanceLoading, setGeminiInstanceLoading] = useState(false);
  const [geminiRefreshLoading, setGeminiRefreshLoading] = useState(false);
  const [geminiEditDialog, setGeminiEditDialog] = useState<GeminiEditDialogState | null>(null);
  const [confirmDeleteGeminiId, setConfirmDeleteGeminiId] = useState<string | null>(null);
  const [confirmWebdavPull, setConfirmWebdavPull] = useState(false);

  return {
    loading,
    setLoading,
    message,
    setMessage,
    updateMsg,
    setUpdateMsg,
    isCheckingUpdate,
    setIsCheckingUpdate,
    linuxInstallBusyUrl,
    setLinuxInstallBusyUrl,
    availableUpdate,
    setAvailableUpdate,
    floatingCardMsg,
    setFloatingCardMsg,
    updateProgress,
    setUpdateProgress,
    webdavUrl,
    setWebdavUrl,
    webdavUser,
    setWebdavUser,
    webdavPass,
    setWebdavPass,
    webdavMsg,
    setWebdavMsg,
    webdavLoading,
    setWebdavLoading,
    geminiInstanceName,
    setGeminiInstanceName,
    geminiInstanceDir,
    setGeminiInstanceDir,
    geminiInstanceMsg,
    setGeminiInstanceMsg,
    geminiInstanceLoading,
    setGeminiInstanceLoading,
    geminiRefreshLoading,
    setGeminiRefreshLoading,
    geminiEditDialog,
    setGeminiEditDialog,
    confirmDeleteGeminiId,
    setConfirmDeleteGeminiId,
    confirmWebdavPull,
    setConfirmWebdavPull,
  };
}

export type SettingsPageState = ReturnType<typeof useSettingsPageState>;
