import type { ChangeEvent, Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check, type Update as TauriUpdate } from "@tauri-apps/plugin-updater";
import {
  api,
  type UpdateRuntimeInfo,
  type UpdateSettings,
} from "../../lib/api";
import type { UpdateProgressState } from "./settingsTypes";

const normalizeReminderStrategy = (value?: string | null) => {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "daily") return "daily";
  if (normalized === "weekly") return "weekly";
  return "immediate";
};

const normalizeSkipVersion = (value?: string | null) => {
  const normalized = String(value || "").trim();
  return normalized || null;
};

const reminderReasonMessage = (reason: string, version: string) => {
  const normalizedVersion = version.trim() || "unknown";
  if (reason === "skipped_version") {
    return `发现版本 ${normalizedVersion}，但该版本已在“跳过版本”策略中。`;
  }
  if (reason === "reminders_disabled") {
    return `发现版本 ${normalizedVersion}，但你已关闭更新提醒。`;
  }
  if (reason === "silent_window_active") {
    return `发现版本 ${normalizedVersion}，已按静默提醒策略延后展示。`;
  }
  if (reason === "invalid_version") {
    return "发现更新，但版本号异常，已跳过提醒。";
  }
  return `发现新版本 ${normalizedVersion}`;
};

type UseSettingsBackupAndUpdateParams = {
  updateSettings: UpdateSettings | null;
  updateRuntimeInfo: UpdateRuntimeInfo | null;
  availableUpdate: TauriUpdate | null;
  webdavUrl: string;
  webdavUser: string;
  webdavPass: string;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setUpdateMsg: Dispatch<SetStateAction<string>>;
  setIsCheckingUpdate: Dispatch<SetStateAction<boolean>>;
  setUpdateSettings: Dispatch<SetStateAction<UpdateSettings | null>>;
  setLinuxInstallBusyUrl: Dispatch<SetStateAction<string | null>>;
  setAvailableUpdate: Dispatch<SetStateAction<TauriUpdate | null>>;
  setUpdateProgress: Dispatch<SetStateAction<UpdateProgressState>>;
  setWebdavMsg: Dispatch<SetStateAction<string>>;
  setWebdavLoading: Dispatch<SetStateAction<boolean>>;
  setConfirmWebdavPull: Dispatch<SetStateAction<boolean>>;
};

export function useSettingsBackupAndUpdate({
  updateSettings,
  updateRuntimeInfo,
  availableUpdate,
  webdavUrl,
  webdavUser,
  webdavPass,
  setLoading,
  setMessage,
  setUpdateMsg,
  setIsCheckingUpdate,
  setUpdateSettings,
  setLinuxInstallBusyUrl,
  setAvailableUpdate,
  setUpdateProgress,
  setWebdavMsg,
  setWebdavLoading,
  setConfirmWebdavPull,
}: UseSettingsBackupAndUpdateParams) {
  const saveWebdavConfig = () => {
    localStorage.setItem("webdav_url", webdavUrl);
    localStorage.setItem("webdav_user", webdavUser);
    localStorage.setItem("webdav_pass", webdavPass);
    const config = { url: webdavUrl, username: webdavUser, password: webdavPass || null };
    invoke("webdav_save_config", { config }).catch(console.error);
    return config;
  };

  const handleInstallUpdate = async (update = availableUpdate) => {
    if (!update) return;
    try {
      let downloaded = 0;
      let total = 0;
      setUpdateProgress({ phase: "downloading", downloaded: 0, total: 0 });
      setUpdateMsg(`正在下载 ${update.version}...`);
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength || 0;
            setUpdateProgress({ phase: "downloading", downloaded: 0, total });
            setUpdateMsg(
              total > 0
                ? `开始下载 ${update.version}（共 ${total} bytes）`
                : `开始下载 ${update.version}`
            );
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            setUpdateProgress({ phase: "downloading", downloaded, total });
            setUpdateMsg(
              total > 0 ? `已下载 ${downloaded} / ${total}` : `已下载 ${downloaded} bytes`
            );
            break;
          case "Finished":
            setUpdateProgress({ phase: "installing", downloaded, total });
            setUpdateMsg("下载完成，正在安装更新...");
            break;
        }
      });
      setUpdateProgress({ phase: "finished", downloaded, total });
      setUpdateMsg("更新已安装！需要重启应用后生效。");
    } catch (e) {
      setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
      setUpdateMsg(`安装更新失败: ${String(e)}`);
    }
  };

  const handleCheckUpdate = async () => {
    setIsCheckingUpdate(true);
    setUpdateProgress({ phase: "checking", downloaded: 0, total: 0 });
    setUpdateMsg("正在请求更新服务器...");
    try {
      const settings = await api.update.markCheckedNow();
      setUpdateSettings(settings);
      const update = await check();
      if (update) {
        const version = String(update.version || "").trim();
        const decision = await api.update.evaluateReminderPolicy(version);
        setUpdateSettings(decision.settings);
        if (decision.should_notify) {
          setAvailableUpdate(update);
          setUpdateMsg(reminderReasonMessage("allow_immediate", version));
          const reminded = await api.update.markReminded(version).catch(() => null);
          if (reminded) {
            setUpdateSettings(reminded);
          }
          if (decision.settings.auto_install && updateRuntimeInfo?.can_auto_install !== false) {
            await handleInstallUpdate(update);
          } else {
            setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
          }
        } else {
          setAvailableUpdate(null);
          setUpdateMsg(reminderReasonMessage(decision.reason, version));
          setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
        }
      } else {
        setAvailableUpdate(null);
        setUpdateMsg("当前已是最新版本");
        setUpdateProgress({ phase: "finished", downloaded: 0, total: 0 });
      }
    } catch (e) {
      setUpdateMsg(`检查更新失败: ${String(e)} (可能是因为测试地址或网络问题)`);
      setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleUpdateSettingChange = async (
    patch: Partial<UpdateSettings>,
    successMessage?: string
  ) => {
    if (!updateSettings) return;
    const next = { ...updateSettings, ...patch };
    next.skip_version = normalizeSkipVersion(next.skip_version);
    next.silent_reminder_strategy = normalizeReminderStrategy(next.silent_reminder_strategy);
    setUpdateSettings(next);
    try {
      await api.update.saveSettings(next);
      if (successMessage) setUpdateMsg(successMessage);
    } catch (e) {
      setUpdateMsg(`保存更新设置失败: ${e}`);
      setUpdateSettings(updateSettings);
    }
  };

  const handleSkipFoundVersion = async () => {
    const version = normalizeSkipVersion(availableUpdate?.version);
    if (!version) {
      setUpdateMsg("当前没有可跳过的目标版本");
      return;
    }
    await handleUpdateSettingChange({ skip_version: version }, `已跳过版本 ${version}`);
    setAvailableUpdate(null);
  };

  const handleClearSkipVersion = async () => {
    await handleUpdateSettingChange({ skip_version: null }, "已清除跳过版本策略");
  };

  const handleInstallLinuxAsset = async (
    url: string,
    kind: string,
    version: string
  ) => {
    try {
      setLinuxInstallBusyUrl(url);
      const result = await api.update.installLinuxAsset({ url, kind, version });
      setUpdateMsg(`${result.message} 路径：${result.downloaded_path}`);
    } catch (e) {
      setUpdateMsg(`Linux 安装执行失败: ${e}`);
    } finally {
      setLinuxInstallBusyUrl(null);
    }
  };

  const handleExport = async () => {
    try {
      setLoading(true);
      setMessage("正在导出配置...");
      const data = await invoke("export_config");
      const jsonStr = JSON.stringify(data, null, 2);
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `ai-singularity-config-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setMessage("配置导出成功！");
    } catch (e) {
      setMessage(`导出失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      setLoading(true);
      setMessage("正在读取文件...");
      const text = await file.text();
      setMessage("正在导入配置...");
      await invoke("import_config", { jsonData: text });
      setMessage("配置导入成功！后台数据已刷新。");
    } catch (err) {
      setMessage(`导入失败: ${err}`);
    } finally {
      setLoading(false);
      event.target.value = "";
    }
  };

  const handleWebdavTest = async () => {
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在测试连接...");
      await api.webdav.testConnection(saveWebdavConfig());
      setWebdavMsg("✅ 测试成功！");
    } catch (e) {
      setWebdavMsg(`❌ 测试失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

  const handleWebdavPush = async () => {
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在推送至云端...");
      await api.webdav.push(saveWebdavConfig());
      setWebdavMsg("✅ 推送同步成功！");
    } catch (e) {
      setWebdavMsg(`❌ 推送失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

  const handleWebdavPull = async () => {
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在从云端拉取配置...");
      await api.webdav.pull(saveWebdavConfig());
      setWebdavMsg("✅ 拉取成功！数据已应用。");
      setConfirmWebdavPull(false);
    } catch (e) {
      setWebdavMsg(`❌ 拉取失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

  const selectedReminderStrategy = normalizeReminderStrategy(
    updateSettings?.silent_reminder_strategy
  );

  return {
    selectedReminderStrategy,
    handleCheckUpdate,
    handleInstallUpdate,
    handleUpdateSettingChange,
    handleSkipFoundVersion,
    handleClearSkipVersion,
    handleInstallLinuxAsset,
    handleExport,
    handleImport,
    handleWebdavTest,
    handleWebdavPush,
    handleWebdavPull,
  };
}

export type SettingsBackupAndUpdateState = ReturnType<typeof useSettingsBackupAndUpdate>;
