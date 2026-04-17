import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import { check, Update as TauriUpdate } from "@tauri-apps/plugin-updater";
import {
  api,
  type CurrentAccountSnapshot,
  type GeminiInstanceRecord,
  type OAuthEnvStatusItem,
  type FloatingAccountCard,
  type LinuxReleaseInfo,
  type SkillStorageInfo,
  type UpdateRuntimeInfo,
  type UpdateSettings,
  type WebReportStatus,
  type WebSocketStatus,
} from "../../lib/api";
import type { IdeAccount } from "../../types";

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

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateSettings, setUpdateSettings] = useState<UpdateSettings | null>(null);
  const [updateRuntimeInfo, setUpdateRuntimeInfo] = useState<UpdateRuntimeInfo | null>(null);
  const [linuxReleaseInfo, setLinuxReleaseInfo] = useState<LinuxReleaseInfo | null>(null);
  const [linuxInstallBusyUrl, setLinuxInstallBusyUrl] = useState<string | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<TauriUpdate | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<WebSocketStatus | null>(null);
  const [webReportStatus, setWebReportStatus] = useState<WebReportStatus | null>(null);
  const [currentSnapshots, setCurrentSnapshots] = useState<CurrentAccountSnapshot[]>([]);
  const [floatingCards, setFloatingCards] = useState<FloatingAccountCard[]>([]);
  const [floatingCardMsg, setFloatingCardMsg] = useState("");
  const [updateProgress, setUpdateProgress] = useState<{ phase: "idle" | "checking" | "downloading" | "installing" | "finished"; downloaded: number; total: number }>({
    phase: "idle",
    downloaded: 0,
    total: 0,
  });

  const [webdavUrl, setWebdavUrl] = useState(localStorage.getItem("webdav_url") || "");
  const [webdavUser, setWebdavUser] = useState(localStorage.getItem("webdav_user") || "");
  const [webdavPass, setWebdavPass] = useState(localStorage.getItem("webdav_pass") || "");
  const [webdavMsg, setWebdavMsg] = useState("");
  const [webdavLoading, setWebdavLoading] = useState(false);
  const [skillStorage, setSkillStorage] = useState<SkillStorageInfo | null>(null);
  const [oauthEnvStatus, setOauthEnvStatus] = useState<OAuthEnvStatusItem[]>([]);
  const [ideAccounts, setIdeAccounts] = useState<IdeAccount[]>([]);
  const [currentGeminiAccountId, setCurrentGeminiAccountId] = useState<string | null>(null);
  const [geminiInstances, setGeminiInstances] = useState<GeminiInstanceRecord[]>([]);
  const [defaultGeminiInstance, setDefaultGeminiInstance] = useState<GeminiInstanceRecord | null>(null);
  const [geminiInstanceName, setGeminiInstanceName] = useState("");
  const [geminiInstanceDir, setGeminiInstanceDir] = useState("");
  const [geminiInstanceMsg, setGeminiInstanceMsg] = useState("");
  const [geminiInstanceLoading, setGeminiInstanceLoading] = useState(false);
  const [geminiRefreshLoading, setGeminiRefreshLoading] = useState(false);
  const [geminiEditDialog, setGeminiEditDialog] = useState<{
    instance: GeminiInstanceRecord;
    extraArgs: string;
    bindAccountId: string;
    projectId: string;
    followLocalAccount: boolean;
  } | null>(null);
  const [confirmDeleteGeminiId, setConfirmDeleteGeminiId] = useState<string | null>(null);
  const [confirmWebdavPull, setConfirmWebdavPull] = useState(false);
  const [runtimeLoading, setRuntimeLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const loadRuntimeInfo = async () => {
      setRuntimeLoading(true);
      try {
        const [storageInfo, oauthInfo, ideAccountsList, snapshots, floatingCardList, instanceList, defaultInstance] = await Promise.all([
          api.skills.getStorageInfo(),
          api.oauth.getEnvStatus(),
          api.ideAccounts.list(),
          api.providerCurrent.listSnapshots(),
          api.floatingCards.list().catch(() => []),
          api.geminiInstances.list(),
          api.geminiInstances.getDefault(),
        ]);
        const [runtimeInfo, savedUpdateSettings] = await Promise.all([
          api.update.getRuntimeInfo(),
          api.update.getSettings(),
        ]);
        const wsStatus = await api.websocket.getStatus();
        const reportStatus = await api.webReport.getStatus().catch(() => null);
        if (!cancelled) {
          setSkillStorage(storageInfo);
          setOauthEnvStatus(oauthInfo);
          setIdeAccounts(ideAccountsList);
          setCurrentSnapshots(snapshots);
          setFloatingCards(floatingCardList);
          setCurrentGeminiAccountId(snapshots.find((item) => item.platform === "gemini")?.account_id ?? null);
          setGeminiInstances(instanceList);
          setDefaultGeminiInstance(defaultInstance);
          setUpdateRuntimeInfo(runtimeInfo);
          setUpdateSettings(savedUpdateSettings);
          setWebsocketStatus(wsStatus);
          setWebReportStatus(reportStatus);
        }
        if (!cancelled && runtimeInfo.platform === "linux") {
          api.update.getLinuxReleaseInfo().then(setLinuxReleaseInfo).catch((error) => {
            console.warn("Failed to load Linux release info:", error);
          });
        }
      } catch (e) {
        if (!cancelled) {
          console.error("Failed to load runtime info:", e);
        }
      } finally {
        if (!cancelled) setRuntimeLoading(false);
      }
    };

    loadRuntimeInfo();
    return () => {
      cancelled = true;
    };
  }, []);

  const reloadGeminiInstances = async () => {
    const [instanceList, defaultInstance, snapshots, accounts] = await Promise.all([
      api.geminiInstances.list(),
      api.geminiInstances.getDefault(),
      api.providerCurrent.listSnapshots().catch(() => []),
      api.ideAccounts.list(),
    ]);
    setIdeAccounts(accounts);
    setCurrentSnapshots(snapshots);
    setCurrentGeminiAccountId(snapshots.find((item) => item.platform === "gemini")?.account_id ?? null);
    setGeminiInstances(instanceList);
    setDefaultGeminiInstance(defaultInstance);
  };

  const reloadFloatingCards = async () => {
    const cards = await api.floatingCards.list().catch(() => []);
    setFloatingCards(cards);
  };

  const handleFloatingCardError = async (error: unknown, fallback = "浮窗操作失败") => {
    if (String(error).includes("floating_card_conflict")) {
      setFloatingCardMsg("浮窗已在其他窗口更新，已刷新到最新状态");
      await reloadFloatingCards();
      return;
    }
    setFloatingCardMsg(`${fallback}: ${error}`);
  };

  const handleCreateGlobalFloatingCard = async () => {
    try {
      await api.floatingCards.create({
        scope: "global",
        title: "全局账号浮窗",
        bound_platforms: ["codex", "gemini"],
        window_label: "main",
      });
      setFloatingCardMsg("已创建全局浮窗");
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "创建全局浮窗失败");
    }
  };

  const handleToggleFloatingCardVisible = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.update(
        card.id,
        { visible: !card.visible },
        card.updated_at
      );
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "更新浮窗可见状态失败");
    }
  };

  const handleToggleFloatingCardTop = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.update(
        card.id,
        { always_on_top: !card.always_on_top },
        card.updated_at
      );
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "更新浮窗置顶状态失败");
    }
  };

  const handleDeleteFloatingCard = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.delete(card.id);
      setFloatingCardMsg("浮窗已删除");
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "删除浮窗失败");
    }
  };

  const geminiAccounts = ideAccounts.filter((item) => item.origin_platform === "gemini");
  const getGeminiAccount = (accountId?: string | null) =>
    accountId ? geminiAccounts.find((item) => item.id === accountId) || null : null;
  const getGeminiAccountLabel = (accountId?: string | null) => {
    if (!accountId) return "未绑定账号";
    const matched = getGeminiAccount(accountId);
    if (!matched) return accountId;
    return matched.label?.trim() || matched.email;
  };
  const isCurrentLocalGeminiAccount = (accountId?: string | null) =>
    !!accountId && !!currentGeminiAccountId && accountId === currentGeminiAccountId;
  const currentGeminiAccountLabel = getGeminiAccountLabel(currentGeminiAccountId);
  const getGeminiAccountProjectLabel = (accountId?: string | null) => {
    const matched = getGeminiAccount(accountId);
    return matched?.project_id?.trim() || null;
  };
  const getEffectiveGeminiAccountId = (instance: GeminiInstanceRecord) =>
    instance.is_default && instance.follow_local_account
      ? currentGeminiAccountId
      : instance.bind_account_id || null;
  const getEffectiveGeminiProjectId = (instance: GeminiInstanceRecord) =>
    instance.project_id?.trim() || getGeminiAccountProjectLabel(getEffectiveGeminiAccountId(instance));
  const formatGeminiLaunchTime = (value?: string | null) =>
    value ? new Date(value).toLocaleString() : "尚未启动";
  const sortedGeminiInstances = [...geminiInstances].sort((a, b) => {
    const aTime = a.last_launched_at ? new Date(a.last_launched_at).getTime() : 0;
    const bTime = b.last_launched_at ? new Date(b.last_launched_at).getTime() : 0;
    if (aTime !== bTime) return bTime - aTime;
    return a.name.localeCompare(b.name, "zh-CN");
  });
  const allVisibleGeminiInstances = defaultGeminiInstance
    ? [defaultGeminiInstance, ...sortedGeminiInstances]
    : sortedGeminiInstances;
  const geminiConflictCount = allVisibleGeminiInstances.filter((instance) => {
    const effectiveAccountId = getEffectiveGeminiAccountId(instance);
    return !!effectiveAccountId && !!currentGeminiAccountId && effectiveAccountId !== currentGeminiAccountId;
  }).length;
  const geminiProjectOverrideCount = allVisibleGeminiInstances.filter((instance) => !!instance.project_id?.trim()).length;
  const geminiUninitializedCount = allVisibleGeminiInstances.filter((instance) => !instance.initialized).length;
  const geminiUnboundCount = allVisibleGeminiInstances.filter((instance) => !getEffectiveGeminiAccountId(instance)).length;
  const getGeminiInstanceWarnings = (instance: GeminiInstanceRecord) => {
    const warnings: Array<{ tone: "warning" | "info" | "success"; text: string }> = [];
    const effectiveAccountId = getEffectiveGeminiAccountId(instance);
    const effectiveProjectId =
      instance.project_id?.trim() || getGeminiAccountProjectLabel(effectiveAccountId);

    if (instance.is_default && instance.follow_local_account) {
      if (!currentGeminiAccountId) {
        warnings.push({
          tone: "warning",
          text: "已开启跟随当前本地账号，但当前没有可解析的 Gemini 本地账号。",
        });
      } else {
        warnings.push({
          tone: "success",
          text: `默认实例启动时会跟随当前本地账号：${getGeminiAccountLabel(currentGeminiAccountId)}。`,
        });
      }
    } else if (!effectiveAccountId) {
      warnings.push({
        tone: "warning",
        text: "当前没有有效绑定账号，启动前无法写入 Gemini 本地凭据。",
      });
    } else if (currentGeminiAccountId && effectiveAccountId !== currentGeminiAccountId) {
      warnings.push({
        tone: "warning",
        text: `启动时会用 ${getGeminiAccountLabel(effectiveAccountId)} 覆盖当前本地账号 ${getGeminiAccountLabel(currentGeminiAccountId)}。`,
      });
    } else if (effectiveAccountId) {
      warnings.push({
        tone: "success",
        text: `启动时会使用 ${getGeminiAccountLabel(effectiveAccountId)}。`,
      });
    }

    if (instance.project_id?.trim()) {
      const accountProject = getGeminiAccountProjectLabel(effectiveAccountId);
      if (accountProject && accountProject !== instance.project_id?.trim()) {
        warnings.push({
          tone: "info",
          text: `实例项目 ${instance.project_id?.trim()} 会覆盖账号默认项目 ${accountProject}。`,
        });
      } else {
        warnings.push({
          tone: "info",
          text: `实例已固定项目 ${instance.project_id?.trim()}。`,
        });
      }
    } else if (effectiveProjectId) {
      warnings.push({
        tone: "info",
        text: `当前将沿用账号项目 ${effectiveProjectId}。`,
      });
    }

    if (!instance.initialized) {
      warnings.push({
        tone: "info",
        text: "该实例目录尚未初始化，首次启动后才会生成本地 .gemini 凭据文件。",
      });
    }

    return warnings;
  };

  const handlePickGeminiDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择 Gemini 实例目录",
    });
    if (typeof selected === "string") {
      setGeminiInstanceDir(selected);
    }
  };

  const handleAddGeminiInstance = async () => {
    if (!geminiInstanceName.trim() || !geminiInstanceDir.trim()) {
      setGeminiInstanceMsg("请填写实例名称并选择目录");
      return;
    }
    setGeminiInstanceLoading(true);
    try {
      await api.geminiInstances.add(geminiInstanceName.trim(), geminiInstanceDir.trim());
      setGeminiInstanceName("");
      setGeminiInstanceDir("");
      setGeminiInstanceMsg("Gemini 实例已添加");
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`添加 Gemini 实例失败: ${e}`);
    } finally {
      setGeminiInstanceLoading(false);
    }
  };

  const handleUpdateGeminiInstance = async (instance: GeminiInstanceRecord) => {
    setGeminiEditDialog({
      instance,
      extraArgs: instance.extra_args || "",
      bindAccountId: instance.bind_account_id || "",
      projectId: instance.project_id || "",
      followLocalAccount: !!instance.follow_local_account,
    });
  };

  const handleQuickUpdateGeminiInstance = async (
    instance: GeminiInstanceRecord,
    patch: {
      extraArgs?: string | null;
      bindAccountId?: string | null;
      projectId?: string | null;
      followLocalAccount?: boolean | null;
    },
    successMessage: string,
  ) => {
    try {
      await api.geminiInstances.update(
        instance.id,
        patch.extraArgs ?? instance.extra_args ?? "",
        patch.bindAccountId !== undefined
          ? patch.bindAccountId
          : instance.bind_account_id ?? null,
        patch.projectId !== undefined
          ? patch.projectId
          : instance.project_id ?? null,
        instance.is_default
          ? (patch.followLocalAccount !== undefined
              ? patch.followLocalAccount
              : instance.follow_local_account ?? false)
          : null,
      );
      setGeminiInstanceMsg(successMessage);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`更新 Gemini 实例失败: ${e}`);
    }
  };

  const editEffectiveGeminiAccountId = geminiEditDialog
    ? geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount
      ? currentGeminiAccountId
      : geminiEditDialog.bindAccountId || null
    : null;
  const editSelectedGeminiProjectId = geminiEditDialog?.projectId.trim() || null;
  const editAccountProjectId = getGeminiAccountProjectLabel(editEffectiveGeminiAccountId);
  const applyGeminiEditPatch = (patch: Partial<NonNullable<typeof geminiEditDialog>>) => {
    setGeminiEditDialog((current) => (current ? { ...current, ...patch } : current));
  };

  const handleCopyGeminiLaunchCommand = async (id: string) => {
    try {
      const info = await api.geminiInstances.getLaunchCommand(id);
      await navigator.clipboard.writeText(info.launch_command);
      setGeminiInstanceMsg("Gemini 启动命令已复制到剪贴板");
    } catch (e) {
      setGeminiInstanceMsg(`读取 Gemini 启动命令失败: ${e}`);
    }
  };

  const handleLaunchGeminiInstance = async (id: string) => {
    try {
      const message = await api.geminiInstances.launch(id);
      setGeminiInstanceMsg(message);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`启动 Gemini 实例失败: ${e}`);
    }
  };

  const handleDeleteGeminiInstance = async (id: string) => {
    try {
      await api.geminiInstances.delete(id);
      setGeminiInstanceMsg("Gemini 实例已删除");
      setConfirmDeleteGeminiId(null);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`删除 Gemini 实例失败: ${e}`);
    }
  };

  const handleRefreshGeminiRuntime = async () => {
    try {
      setGeminiRefreshLoading(true);
      const refreshed = await api.ideAccounts.refreshAllByPlatform("gemini");
      await reloadGeminiInstances();
      setGeminiInstanceMsg(`Gemini 账号状态已刷新，共处理 ${refreshed} 个账号`);
    } catch (e) {
      setGeminiInstanceMsg(`刷新 Gemini 账号状态失败: ${e}`);
    } finally {
      setGeminiRefreshLoading(false);
    }
  };

  const saveWebdavConfig = () => {
    localStorage.setItem("webdav_url", webdavUrl);
    localStorage.setItem("webdav_user", webdavUser);
    localStorage.setItem("webdav_pass", webdavPass);
    const config = { url: webdavUrl, username: webdavUser, password: webdavPass || null };
    // Send to backend for daemon usage
    invoke("webdav_save_config", { config }).catch(console.error);
    return config;
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

  const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const lang = e.target.value;
    i18n.changeLanguage(lang);
    localStorage.setItem("ais_lang", lang);
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
            setUpdateMsg(total > 0 ? `开始下载 ${update.version}（共 ${total} bytes）` : `开始下载 ${update.version}`);
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            setUpdateProgress({ phase: "downloading", downloaded, total });
            setUpdateMsg(total > 0 ? `已下载 ${downloaded} / ${total}` : `已下载 ${downloaded} bytes`);
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

  const handleUpdateSettingChange = async (
    patch: Partial<UpdateSettings>,
    successMessage?: string,
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
    await handleUpdateSettingChange(
      { skip_version: version },
      `已跳过版本 ${version}`
    );
    setAvailableUpdate(null);
  };

  const handleClearSkipVersion = async () => {
    await handleUpdateSettingChange(
      { skip_version: null },
      "已清除跳过版本策略"
    );
  };

  const selectedReminderStrategy = normalizeReminderStrategy(
    updateSettings?.silent_reminder_strategy
  );

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

  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      setLoading(true);
      setMessage("正在读取文件...");
      const text = await file.text();
      
      setMessage("正在导入配置...");
      await invoke("import_config", { jsonData: text });
      setMessage("配置导入成功！后台数据已刷新。");
      // 可以在此处提示重启，或依靠前端其他机制拉取最新状态
    } catch (err) {
      setMessage(`导入失败: ${err}`);
    } finally {
      setLoading(false);
      // reset input
      e.target.value = "";
    }
  };

  return (
    <div>
      <div className="page-header">
        <div>
          <h1 className="page-title">{t("settings.title")}</h1>
          <p className="page-subtitle">{t("settings.subtitle")}</p>
        </div>
      </div>

      <div className="settings-section" style={{ padding: "var(--space-6)" }}>
        <h3 style={{ marginBottom: "var(--space-2)" }}>{t("settings.language")}</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>{t("settings.language_desc")}</p>
        <div style={{ marginBottom: "var(--space-6)" }}>
          <select 
            className="form-input" 
            style={{ width: "200px" }}
            value={i18n.language.startsWith("zh") ? "zh" : "en"}
            onChange={handleLanguageChange}
          >
            <option value="zh">简体中文</option>
            <option value="en">English</option>
          </select>
        </div>

        <h3 style={{ marginBottom: "var(--space-2)" }}>运行时状态</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          把当前应用真正依赖的本地路径和 OAuth 环境配置显式展示出来，方便排查问题。
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>Skills 存储位置</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : skillStorage ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>主仓路径</div>
                  <code style={{ fontSize: 12, wordBreak: "break-all" }}>{skillStorage.primary_path}</code>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>兼容旧目录</div>
                  <code style={{ fontSize: 12, wordBreak: "break-all" }}>{skillStorage.legacy_path}</code>
                </div>
                <div style={{ fontSize: 12, color: skillStorage.legacy_exists ? "var(--color-warning)" : "var(--color-success)" }}>
                  {skillStorage.legacy_exists ? "检测到旧目录，应用会继续兼容读取。" : "未检测到旧目录，当前已完全使用新技能仓。"}
                </div>
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>未能读取技能仓信息</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>OAuth 环境配置</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : oauthEnvStatus.length > 0 ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                {oauthEnvStatus.map((item) => (
                  <div key={item.env_name} style={{ paddingBottom: "var(--space-2)", borderBottom: "1px solid var(--color-border)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                      <span style={{ fontWeight: 500, fontSize: 13 }}>{item.provider}</span>
                      <span style={{ fontSize: 12, color: item.configured ? "var(--color-success)" : "var(--color-danger)" }}>
                        {item.configured ? "已配置" : "缺失"}
                      </span>
                    </div>
                    <code style={{ fontSize: 12, wordBreak: "break-all" }}>{item.env_name}</code>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>当前没有需要展示的 OAuth 环境项</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>本地 WebSocket 广播</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : websocketStatus ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div style={{ fontSize: 13 }}>
                  状态：
                  <strong style={{ marginLeft: 6 }}>{websocketStatus.running ? "运行中" : "未启动"}</strong>
                </div>
                <div style={{ fontSize: 13 }}>
                  端口：
                  <code style={{ marginLeft: 6 }}>{websocketStatus.port ?? "—"}</code>
                </div>
                <div style={{ fontSize: 13 }}>
                  客户端数：
                  <strong style={{ marginLeft: 6 }}>{websocketStatus.client_count}</strong>
                </div>
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>未能读取 WebSocket 状态</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>本地 Web Report 状态页</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : webReportStatus ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div style={{ fontSize: 13 }}>
                  地址：
                  <code style={{ marginLeft: 6 }}>
                    {webReportStatus.local_url || "—"}
                  </code>
                </div>
                <div style={{ fontSize: 13 }}>
                  健康检查：
                  <code style={{ marginLeft: 6 }}>
                    {webReportStatus.health_url || "—"}
                  </code>
                </div>
                <div style={{ fontSize: 13 }}>
                  JSON 状态：
                  <code style={{ marginLeft: 6 }}>
                    {webReportStatus.status_api_url || "—"}
                  </code>
                </div>
                <div style={{ fontSize: 13 }}>
                  JSON 快照：
                  <code style={{ marginLeft: 6 }}>
                    {webReportStatus.snapshot_api_url || "—"}
                  </code>
                </div>
                <div style={{ fontSize: 13 }}>
                  JSON 认证：
                  <strong style={{ marginLeft: 6 }}>
                    {webReportStatus.auth_enabled ? "已启用（需携带 token）" : "未启用"}
                  </strong>
                </div>
                <div className="text-muted" style={{ fontSize: 12 }}>
                  现在除了 HTML 状态页，也能给外部客户端直接消费 `status/snapshot` JSON；如配置环境变量 `AIS_WEB_REPORT_TOKEN`，JSON 接口会要求认证。
                </div>
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div className="text-muted" style={{ fontSize: 13 }}>未能读取 Web Report 状态</div>
              </div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>当前账号快照</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : currentSnapshots.length > 0 ? (
              <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12 }}>
                {currentSnapshots.map((item) => (
                  <div
                    key={item.platform}
                    style={{
                      padding: "10px 12px",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--color-border)",
                      background: "rgba(255,255,255,0.04)",
                    }}
                  >
                    <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 4 }}>{item.platform}</div>
                    <div style={{ fontSize: 13 }}>{item.label || "未解析到当前账号"}</div>
                    <div className="text-muted" style={{ fontSize: 11, marginTop: 4 }}>
                      {item.email || "—"} · {item.status || "unknown"}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>当前没有可展示的账号快照</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <div style={{ fontWeight: 600 }}>浮动账号卡片</div>
              <button className="btn btn-secondary" onClick={handleCreateGlobalFloatingCard}>
                新建全局浮窗
              </button>
            </div>
            <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
              浮窗支持实例绑定、拖拽定位记忆和跨窗口同步，账号切换会自动广播到所有窗口。
            </div>
            <div style={{ marginTop: 10, display: "flex", flexDirection: "column", gap: 8 }}>
              {floatingCards.length === 0 ? (
                <div className="text-muted" style={{ fontSize: 13 }}>当前还没有浮动账号卡片</div>
              ) : (
                floatingCards.map((card) => (
                  <div
                    key={card.id}
                    style={{
                      padding: "10px 12px",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--color-border)",
                      background: "rgba(255,255,255,0.04)",
                      display: "flex",
                      justifyContent: "space-between",
                      gap: 8,
                      alignItems: "center",
                      flexWrap: "wrap",
                    }}
                  >
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600 }}>{card.title}</div>
                      <div className="text-muted" style={{ fontSize: 11, marginTop: 4 }}>
                        {card.scope === "instance"
                          ? `实例绑定: ${card.instance_id || "未知实例"}`
                          : "全局浮窗"}
                        {" · "}
                        {`平台 ${card.bound_platforms?.join(", ") || "codex, gemini"}`}
                        {" · "}
                        {card.visible ? "可见" : "已隐藏"}
                        {" · "}
                        {card.always_on_top ? "置顶" : "普通"}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <button className="btn btn-secondary" onClick={() => void handleToggleFloatingCardVisible(card)}>
                        {card.visible ? "隐藏" : "显示"}
                      </button>
                      <button className="btn btn-secondary" onClick={() => void handleToggleFloatingCardTop(card)}>
                        {card.always_on_top ? "取消置顶" : "设为置顶"}
                      </button>
                      <button className="btn btn-danger" onClick={() => void handleDeleteFloatingCard(card)}>
                        删除
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
            {floatingCardMsg && (
              <div className="text-muted" style={{ fontSize: 12, marginTop: 8 }}>
                {floatingCardMsg}
              </div>
            )}
          </div>
        </div>

        <h3 style={{ marginBottom: "var(--space-2)" }}>Gemini 实例</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          管理 Gemini CLI 的默认实例与额外实例目录，支持实例级绑定账号、项目 ID 和启动参数。
        </p>
        <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(5, minmax(120px, 1fr))", gap: "var(--space-3)", flex: "1 1 760px" }}>
              <div style={{ padding: "10px 12px", borderRadius: "var(--radius-sm)", background: "rgba(255,255,255,0.04)" }}>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>实例总数</div>
                <div style={{ fontWeight: 700, fontSize: 20 }}>{allVisibleGeminiInstances.length}</div>
              </div>
              <div style={{ padding: "10px 12px", borderRadius: "var(--radius-sm)", background: "rgba(16,185,129,0.10)" }}>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>已初始化</div>
                <div style={{ fontWeight: 700, fontSize: 20 }}>{allVisibleGeminiInstances.length - geminiUninitializedCount}</div>
              </div>
              <div style={{ padding: "10px 12px", borderRadius: "var(--radius-sm)", background: "rgba(245,158,11,0.10)" }}>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>账号冲突</div>
                <div style={{ fontWeight: 700, fontSize: 20 }}>{geminiConflictCount}</div>
              </div>
              <div style={{ padding: "10px 12px", borderRadius: "var(--radius-sm)", background: "rgba(59,130,246,0.10)" }}>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>项目覆盖</div>
                <div style={{ fontWeight: 700, fontSize: 20 }}>{geminiProjectOverrideCount}</div>
              </div>
              <div style={{ padding: "10px 12px", borderRadius: "var(--radius-sm)", background: "rgba(239,68,68,0.10)" }}>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>无有效账号</div>
                <div style={{ fontWeight: 700, fontSize: 20 }}>{geminiUnboundCount}</div>
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
              <button className="btn btn-secondary" onClick={handleRefreshGeminiRuntime} disabled={geminiRefreshLoading}>
                {geminiRefreshLoading ? "刷新中..." : "刷新 Gemini 账号状态"}
              </button>
            </div>
          </div>

          {defaultGeminiInstance && (
            <div style={{ padding: "var(--space-3)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "rgba(255,255,255,0.02)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)" }}>
                <div>
                  <div style={{ fontWeight: 600 }}>默认实例</div>
                  <div className="text-muted" style={{ fontSize: 12, wordBreak: "break-all" }}>{defaultGeminiInstance.user_data_dir}</div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
                    <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(59,130,246,0.12)", color: "var(--color-primary)" }}>
                      当前本地账号：{currentGeminiAccountLabel}
                    </span>
                    <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(255,255,255,0.08)", color: "var(--color-text-secondary)" }}>
                      实际生效账号：{getGeminiAccountLabel(getEffectiveGeminiAccountId(defaultGeminiInstance))}
                    </span>
                    <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(255,255,255,0.08)", color: "var(--color-text-secondary)" }}>
                      实际生效项目：{getEffectiveGeminiProjectId(defaultGeminiInstance) || "沿用本地默认行为"}
                    </span>
                    <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: defaultGeminiInstance.follow_local_account ? "rgba(16,185,129,0.12)" : "rgba(148,163,184,0.16)", color: defaultGeminiInstance.follow_local_account ? "var(--color-success)" : "var(--color-text-secondary)" }}>
                      {defaultGeminiInstance.follow_local_account ? "跟随当前本地账号" : "固定绑定模式"}
                    </span>
                    {defaultGeminiInstance.bind_account_id && (
                      <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: isCurrentLocalGeminiAccount(defaultGeminiInstance.bind_account_id) ? "rgba(16,185,129,0.12)" : "rgba(255,255,255,0.08)", color: isCurrentLocalGeminiAccount(defaultGeminiInstance.bind_account_id) ? "var(--color-success)" : "var(--color-text-secondary)" }}>
                        绑定账号：{getGeminiAccountLabel(defaultGeminiInstance.bind_account_id)}
                        {isCurrentLocalGeminiAccount(defaultGeminiInstance.bind_account_id) ? " · 当前本地" : ""}
                      </span>
                    )}
                  </div>
                  <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                    {defaultGeminiInstance.follow_local_account
                      ? `跟随当前本地账号 (${getGeminiAccountLabel(currentGeminiAccountId)})`
                      : `绑定账号 ${getGeminiAccountLabel(defaultGeminiInstance.bind_account_id)}`}
                    {" · "}
                    {defaultGeminiInstance.follow_local_account ? "跟随当前本地账号" : "固定绑定模式"}
                    {" · "}
                    {defaultGeminiInstance.project_id ? `项目 ${defaultGeminiInstance.project_id}` : "无项目覆盖"}
                    {" · "}
                    {defaultGeminiInstance.extra_args ? `参数 ${defaultGeminiInstance.extra_args}` : "无额外参数"}
                    {" · "}
                    {`最近启动 ${formatGeminiLaunchTime(defaultGeminiInstance.last_launched_at)}`}
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 10 }}>
                    {getGeminiInstanceWarnings(defaultGeminiInstance).map((item, index) => (
                      <div
                        key={`default-warning-${index}`}
                        style={{
                          padding: "8px 10px",
                          borderRadius: "var(--radius-sm)",
                          fontSize: 12,
                          lineHeight: 1.5,
                          background:
                            item.tone === "warning"
                              ? "rgba(245,158,11,0.12)"
                              : item.tone === "success"
                                ? "rgba(16,185,129,0.12)"
                                : "rgba(59,130,246,0.12)",
                          color:
                            item.tone === "warning"
                              ? "var(--color-warning)"
                              : item.tone === "success"
                                ? "var(--color-success)"
                                : "var(--color-primary)",
                        }}
                      >
                        {item.text}
                      </div>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
                    {!defaultGeminiInstance.follow_local_account && currentGeminiAccountId && (
                      <button
                        className="btn btn-secondary"
                        onClick={() =>
                          handleQuickUpdateGeminiInstance(
                            defaultGeminiInstance,
                            { bindAccountId: null, followLocalAccount: true },
                            "默认 Gemini 实例已改为跟随当前本地账号",
                          )
                        }
                      >
                        跟随当前本地账号
                      </button>
                    )}
                    {!defaultGeminiInstance.follow_local_account &&
                      currentGeminiAccountId &&
                      defaultGeminiInstance.bind_account_id !== currentGeminiAccountId && (
                        <button
                          className="btn btn-secondary"
                          onClick={() =>
                            handleQuickUpdateGeminiInstance(
                              defaultGeminiInstance,
                              { bindAccountId: currentGeminiAccountId, followLocalAccount: false },
                              "默认 Gemini 实例已绑定当前本地账号",
                            )
                          }
                        >
                          绑定当前本地账号
                        </button>
                      )}
                    {defaultGeminiInstance.project_id && (
                      <button
                        className="btn btn-secondary"
                        onClick={() =>
                          handleQuickUpdateGeminiInstance(
                            defaultGeminiInstance,
                            { projectId: null },
                            "默认 Gemini 实例已清除项目覆盖",
                          )
                        }
                      >
                        清除项目覆盖
                      </button>
                    )}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                  <button className="btn btn-secondary" onClick={() => handleUpdateGeminiInstance(defaultGeminiInstance)}>设置</button>
                  <button className="btn btn-secondary" onClick={() => handleCopyGeminiLaunchCommand(defaultGeminiInstance.id)}>复制命令</button>
                  <button className="btn btn-primary" onClick={() => handleLaunchGeminiInstance(defaultGeminiInstance.id)}>在终端启动</button>
                </div>
              </div>
            </div>
          )}

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr auto", gap: "var(--space-3)", alignItems: "end" }}>
            <div>
              <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例名称</label>
              <input
                type="text"
                className="form-input"
                value={geminiInstanceName}
                onChange={(e) => setGeminiInstanceName(e.target.value)}
                placeholder="例如：工作区实例 / 沙盒实例"
              />
            </div>
            <div>
              <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例目录</label>
              <input
                type="text"
                className="form-input"
                value={geminiInstanceDir}
                onChange={(e) => setGeminiInstanceDir(e.target.value)}
                placeholder="选择或粘贴实例目录"
              />
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-secondary" onClick={handlePickGeminiDir}>浏览</button>
              <button className="btn btn-primary" onClick={handleAddGeminiInstance} disabled={geminiInstanceLoading}>
                {geminiInstanceLoading ? "添加中..." : "添加实例"}
              </button>
            </div>
          </div>

          {geminiInstanceMsg && (
            <div style={{ padding: "var(--space-2)", background: "rgba(0,0,0,0.2)", borderRadius: "var(--radius-sm)", fontSize: 13 }}>
              {geminiInstanceMsg}
            </div>
          )}

          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            {geminiInstances.length === 0 ? (
              <div className="text-muted" style={{ fontSize: 13 }}>当前还没有额外 Gemini 实例</div>
            ) : (
              sortedGeminiInstances.map((instance) => (
                <div key={instance.id} style={{ padding: "var(--space-3)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "rgba(255,255,255,0.02)" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)" }}>
                    <div>
                      <div style={{ fontWeight: 600 }}>{instance.name}</div>
                      <div className="text-muted" style={{ fontSize: 12, wordBreak: "break-all" }}>{instance.user_data_dir}</div>
                      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
                        {instance.bind_account_id ? (
                          <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: isCurrentLocalGeminiAccount(instance.bind_account_id) ? "rgba(16,185,129,0.12)" : "rgba(255,255,255,0.08)", color: isCurrentLocalGeminiAccount(instance.bind_account_id) ? "var(--color-success)" : "var(--color-text-secondary)" }}>
                            绑定账号：{getGeminiAccountLabel(instance.bind_account_id)}
                            {isCurrentLocalGeminiAccount(instance.bind_account_id) ? " · 当前本地" : ""}
                          </span>
                        ) : (
                          <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(255,255,255,0.08)", color: "var(--color-text-secondary)" }}>
                            未绑定账号
                          </span>
                        )}
                        <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(255,255,255,0.08)", color: "var(--color-text-secondary)" }}>
                          实际生效账号：{getGeminiAccountLabel(getEffectiveGeminiAccountId(instance))}
                        </span>
                        <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: "rgba(255,255,255,0.08)", color: "var(--color-text-secondary)" }}>
                          实际生效项目：{getEffectiveGeminiProjectId(instance) || "沿用本地默认行为"}
                        </span>
                        <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 999, background: instance.initialized ? "rgba(16,185,129,0.12)" : "rgba(245,158,11,0.12)", color: instance.initialized ? "var(--color-success)" : "var(--color-warning)" }}>
                          {instance.initialized ? "已初始化" : "未初始化"}
                        </span>
                      </div>
                      <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                        {instance.bind_account_id ? `绑定账号 ${getGeminiAccountLabel(instance.bind_account_id)}` : "未绑定账号"}
                        {" · "}
                        {instance.project_id ? `项目 ${instance.project_id}` : "无项目覆盖"}
                        {" · "}
                        {instance.extra_args ? `参数 ${instance.extra_args}` : "无额外参数"}
                        {" · "}
                        {instance.initialized ? "已初始化" : "未初始化"}
                        {" · "}
                        {`最近启动 ${formatGeminiLaunchTime(instance.last_launched_at)}`}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 10 }}>
                        {getGeminiInstanceWarnings(instance).map((item, index) => (
                          <div
                            key={`${instance.id}-warning-${index}`}
                            style={{
                              padding: "8px 10px",
                              borderRadius: "var(--radius-sm)",
                              fontSize: 12,
                              lineHeight: 1.5,
                              background:
                                item.tone === "warning"
                                  ? "rgba(245,158,11,0.12)"
                                  : item.tone === "success"
                                    ? "rgba(16,185,129,0.12)"
                                    : "rgba(59,130,246,0.12)",
                              color:
                                item.tone === "warning"
                                  ? "var(--color-warning)"
                                  : item.tone === "success"
                                    ? "var(--color-success)"
                                    : "var(--color-primary)",
                            }}
                          >
                            {item.text}
                          </div>
                        ))}
                      </div>
                      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
                        {currentGeminiAccountId && instance.bind_account_id !== currentGeminiAccountId && (
                          <button
                            className="btn btn-secondary"
                            onClick={() =>
                              handleQuickUpdateGeminiInstance(
                                instance,
                                { bindAccountId: currentGeminiAccountId },
                                `${instance.name} 已绑定当前本地账号`,
                              )
                            }
                          >
                            绑定当前本地账号
                          </button>
                        )}
                        {instance.project_id && (
                          <button
                            className="btn btn-secondary"
                            onClick={() =>
                              handleQuickUpdateGeminiInstance(
                                instance,
                                { projectId: null },
                                `${instance.name} 已清除项目覆盖`,
                              )
                            }
                          >
                            清除项目覆盖
                          </button>
                        )}
                        {!instance.project_id && getGeminiAccountProjectLabel(instance.bind_account_id) && (
                          <button
                            className="btn btn-secondary"
                            onClick={() =>
                              handleQuickUpdateGeminiInstance(
                                instance,
                                { projectId: getGeminiAccountProjectLabel(instance.bind_account_id) },
                                `${instance.name} 已固定为账号默认项目`,
                              )
                            }
                          >
                            固定为账号默认项目
                          </button>
                        )}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                      <button className="btn btn-secondary" onClick={() => handleUpdateGeminiInstance(instance)}>设置</button>
                      <button className="btn btn-secondary" onClick={() => handleCopyGeminiLaunchCommand(instance.id)}>复制命令</button>
                      <button className="btn btn-primary" onClick={() => handleLaunchGeminiInstance(instance.id)}>在终端启动</button>
                      <button className="btn btn-danger" onClick={() => setConfirmDeleteGeminiId(instance.id)}>删除</button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <h3 style={{ marginBottom: "var(--space-4)" }}>配置与备份</h3>
        <p style={{ color: "var(--color-text-secondary)", marginBottom: "var(--space-4)" }}>
          将所有的 Provider、API Key、MCP 以及 Prompt 导出为一个独立文件，方便多端同步或备份。
        </p>

        <div style={{ display: "flex", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
          <button 
            className="btn btn-primary" 
            onClick={handleExport}
            disabled={loading}
          >
            导出配置
          </button>
          
          <label className="btn btn-secondary" style={{ cursor: loading ? "not-allowed" : "pointer" }}>
            导入配置
            <input 
              type="file" 
              accept=".json" 
              style={{ display: "none" }} 
              onChange={handleImport}
              disabled={loading}
            />
          </label>
        </div>

        {message && (
          <div style={{ padding: "var(--space-2)", background: "var(--surface-sunken)", borderRadius: "var(--radius-sm)", fontSize: 14, marginBottom: "var(--space-6)" }}>
            {message}
          </div>
        )}

        {/* WebDAV 同步区 */}
        <h3 style={{ marginBottom: "var(--space-2)" }}>多端 WebDAV 备份同步</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          保障您的配置、Prompt、工具与资产跨端实时同步，数据安全不丢失（原生只支持基于 HTTP 基本认证的 WebDAV）。
        </p>
        <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          <div>
            <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>WebDAV 服务器地址 (URL)</label>
            <input 
              type="text" 
              className="form-input" 
              placeholder="https://dav.your-server.com/" 
              value={webdavUrl}
              onChange={(e) => setWebdavUrl(e.target.value)}
            />
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-3)" }}>
            <div>
              <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>用户名</label>
              <input 
                type="text" 
                className="form-input" 
                value={webdavUser}
                onChange={(e) => setWebdavUser(e.target.value)}
              />
            </div>
            <div>
              <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>密码 / API Token</label>
              <input 
                type="password" 
                className="form-input" 
                value={webdavPass}
                onChange={(e) => setWebdavPass(e.target.value)}
              />
            </div>
          </div>
          <div style={{ display: "flex", gap: "var(--space-3)", marginTop: "var(--space-2)" }}>
            <button className="btn btn-secondary" onClick={handleWebdavTest} disabled={webdavLoading || !webdavUrl}>
              测试连接
            </button>
            <button className="btn btn-primary" onClick={handleWebdavPush} disabled={webdavLoading || !webdavUrl}>
              立即上传推送 (Push)
            </button>
            <button className="btn btn-danger" onClick={() => setConfirmWebdavPull(true)} disabled={webdavLoading || !webdavUrl}>
              立即下载覆盖 (Pull)
            </button>
          </div>
          {webdavMsg && (
            <div style={{ padding: "var(--space-2)", background: "rgba(0,0,0,0.2)", borderRadius: "var(--radius-sm)", fontSize: 13, marginTop: "var(--space-2)" }}>
              {webdavMsg}
            </div>
          )}
        </div>

        {/* 自动更新区 */}
        <h3 style={{ marginBottom: "var(--space-2)" }}>{t("settings.auto_update")}</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>{t("settings.auto_update_desc")}</p>
        <div style={{ marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "var(--space-3)" }}>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>当前版本</div>
              <div style={{ fontWeight: 600 }}>{updateRuntimeInfo?.current_version || "—"}</div>
            </div>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>平台</div>
              <div style={{ fontWeight: 600 }}>
                {updateRuntimeInfo?.platform || "—"}
                {updateRuntimeInfo?.linux_install_kind ? ` · ${updateRuntimeInfo.linux_install_kind}` : ""}
              </div>
            </div>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>上次检查</div>
              <div style={{ fontWeight: 600 }}>
                {updateSettings?.last_check_at ? new Date(updateSettings.last_check_at).toLocaleString() : "尚未检查"}
              </div>
            </div>
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
              <input
                type="checkbox"
                checked={!!updateSettings?.auto_check}
                onChange={(e) => handleUpdateSettingChange({ auto_check: e.target.checked }, "更新设置已保存")}
              />
              自动检查更新
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13, opacity: updateRuntimeInfo?.can_auto_install === false ? 0.65 : 1 }}>
              <input
                type="checkbox"
                checked={!!updateSettings?.auto_install}
                disabled={updateRuntimeInfo?.can_auto_install === false}
                onChange={(e) => handleUpdateSettingChange({ auto_install: e.target.checked }, "更新设置已保存")}
              />
              自动安装更新
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
              <input
                type="checkbox"
                checked={!!updateSettings?.disable_reminders}
                onChange={(e) =>
                  handleUpdateSettingChange(
                    { disable_reminders: e.target.checked },
                    e.target.checked ? "已关闭更新提醒" : "已恢复更新提醒"
                  )
                }
              />
              关闭更新提醒（仅手动检查时显示结果）
            </label>
            <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 13 }}>
              <span>静默提醒策略</span>
              <select
                className="form-input"
                value={selectedReminderStrategy}
                onChange={(e) =>
                  handleUpdateSettingChange(
                    { silent_reminder_strategy: e.target.value },
                    "静默提醒策略已保存"
                  )
                }
                disabled={!!updateSettings?.disable_reminders}
              >
                <option value="immediate">即时提醒（每次命中都提示）</option>
                <option value="daily">每日一次（同版本 24 小时内不重复提醒）</option>
                <option value="weekly">每周一次（同版本 7 天内不重复提醒）</option>
              </select>
            </label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr auto auto", gap: 8, alignItems: "end" }}>
              <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 13 }}>
                <span>跳过指定版本</span>
                <input
                  className="form-input"
                  value={updateSettings?.skip_version || ""}
                  placeholder="例如 0.1.12"
                  onChange={(e) =>
                    handleUpdateSettingChange(
                      { skip_version: e.target.value || null },
                      "跳过版本策略已保存"
                    )
                  }
                />
              </label>
              <button
                className="btn btn-secondary"
                onClick={handleSkipFoundVersion}
                disabled={!availableUpdate?.version}
              >
                跳过当前发现版本
              </button>
              <button
                className="btn btn-secondary"
                onClick={handleClearSkipVersion}
                disabled={!updateSettings?.skip_version}
              >
                清除跳过
              </button>
            </div>
            <div className="text-muted" style={{ fontSize: 12 }}>
              当前策略：
              {updateSettings?.disable_reminders
                ? " 已关闭提醒"
                : ` ${selectedReminderStrategy === "weekly" ? "每周一次" : selectedReminderStrategy === "daily" ? "每日一次" : "即时提醒"}`}
              {updateSettings?.skip_version ? ` · 已跳过 ${updateSettings.skip_version}` : " · 未设置跳过版本"}
            </div>
            {updateRuntimeInfo?.warning && (
              <div className="alert alert-info" style={{ fontSize: 13 }}>
                {updateRuntimeInfo.warning}
              </div>
            )}
            {updateRuntimeInfo?.linux_manual_hint && (
              <div style={{ padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)", fontSize: 13, lineHeight: 1.6 }}>
                <div style={{ fontWeight: 600, marginBottom: 6 }}>Linux 安装处理建议</div>
                <div>{updateRuntimeInfo.linux_manual_hint}</div>
              </div>
            )}
            {updateRuntimeInfo?.platform === "linux" && linuxReleaseInfo && linuxReleaseInfo.assets.length > 0 && (
              <div style={{ padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)", display: "flex", flexDirection: "column", gap: 10 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
                  <div>
                    <div style={{ fontWeight: 600 }}>Linux 发行包资产</div>
                    <div className="text-muted" style={{ fontSize: 12 }}>
                      {linuxReleaseInfo.version}
                      {linuxReleaseInfo.published_at ? ` · ${new Date(linuxReleaseInfo.published_at).toLocaleString()}` : ""}
                    </div>
                  </div>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {linuxReleaseInfo.assets.map((asset) => (
                    <div key={asset.url} style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap", padding: "10px 12px", borderRadius: "var(--radius-sm)", border: "1px solid var(--color-border)" }}>
                      <div>
                        <div style={{ fontWeight: 500 }}>
                          {asset.name}
                          {asset.preferred ? " · 推荐" : ""}
                        </div>
                        <div className="text-muted" style={{ fontSize: 12 }}>
                          {asset.kind}
                          {typeof asset.size === "number" ? ` · ${Math.round(asset.size / 1024)} KB` : ""}
                        </div>
                      </div>
                      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                        <button className="btn btn-secondary" onClick={() => api.update.openAssetUrl(asset.url)}>
                          下载此安装包
                        </button>
                        <button
                          className="btn btn-primary"
                          disabled={linuxInstallBusyUrl === asset.url}
                          onClick={async () => {
                            try {
                              setLinuxInstallBusyUrl(asset.url);
                              const result = await api.update.installLinuxAsset({
                                url: asset.url,
                                kind: asset.kind,
                                version: linuxReleaseInfo.version,
                              });
                              setUpdateMsg(`${result.message} 路径：${result.downloaded_path}`);
                            } catch (e) {
                              setUpdateMsg(`Linux 安装执行失败: ${e}`);
                            } finally {
                              setLinuxInstallBusyUrl(null);
                            }
                          }}
                        >
                          {linuxInstallBusyUrl === asset.url ? "处理中..." : (asset.kind === "appimage" ? "下载并准备" : "下载并安装")}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>Updater Endpoints</div>
              {updateRuntimeInfo?.updater_endpoints?.length ? (
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {updateRuntimeInfo.updater_endpoints.map((endpoint) => (
                    <code key={endpoint} style={{ fontSize: 12, wordBreak: "break-all" }}>{endpoint}</code>
                  ))}
                </div>
              ) : (
                <div className="text-muted" style={{ fontSize: 13 }}>当前未读取到更新地址</div>
              )}
            </div>
          </div>

          <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "center", flexWrap: "wrap" }}>
          <button 
            className="btn btn-primary"
            onClick={handleCheckUpdate}
            disabled={isCheckingUpdate}
          >
            {isCheckingUpdate ? "检查中..." : t("settings.check_now")}
          </button>
          <span className="text-muted" style={{ fontSize: 12 }}>
            {updateRuntimeInfo?.updater_pubkey_configured ? "Updater 公钥已配置" : "Updater 公钥仍为占位值"}
          </span>
          </div>
          {availableUpdate && (
            <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "var(--space-3)" }}>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>发现版本</div>
                  <div style={{ fontWeight: 600 }}>{availableUpdate.version}</div>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>发布日期</div>
                  <div style={{ fontWeight: 600 }}>
                    {availableUpdate.date ? new Date(availableUpdate.date).toLocaleString() : "未知"}
                  </div>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>安装能力</div>
                  <div style={{ fontWeight: 600 }}>
                    {updateRuntimeInfo?.can_auto_install ? "支持插件自动安装" : "建议手动处理"}
                  </div>
                </div>
              </div>

              {availableUpdate.body && (
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 6 }}>发布说明</div>
                  <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: 12, lineHeight: 1.5, padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)" }}>
                    {availableUpdate.body}
                  </pre>
                </div>
              )}

              {updateProgress.phase !== "idle" && (
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                    <span>
                      {updateProgress.phase === "checking" && "正在检查更新"}
                      {updateProgress.phase === "downloading" && "正在下载更新包"}
                      {updateProgress.phase === "installing" && "正在安装更新"}
                      {updateProgress.phase === "finished" && "更新流程完成"}
                    </span>
                    <span>
                      {updateProgress.total > 0 ? `${updateProgress.downloaded} / ${updateProgress.total}` : updateProgress.downloaded > 0 ? `${updateProgress.downloaded} bytes` : ""}
                    </span>
                  </div>
                  <div style={{ height: 8, borderRadius: 999, background: "rgba(255,255,255,0.08)", overflow: "hidden" }}>
                    <div
                      style={{
                        height: "100%",
                        width: updateProgress.total > 0 ? `${Math.min(100, (updateProgress.downloaded / updateProgress.total) * 100)}%` : updateProgress.phase === "finished" ? "100%" : "20%",
                        background: "linear-gradient(90deg, var(--color-primary), var(--color-accent))",
                        transition: "width 180ms ease",
                      }}
                    />
                  </div>
                </div>
              )}

              <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap" }}>
                <button
                  className="btn btn-primary"
                  disabled={isCheckingUpdate || updateProgress.phase === "downloading" || updateProgress.phase === "installing" || updateRuntimeInfo?.can_auto_install === false}
                  onClick={() => handleInstallUpdate()}
                >
                  {updateProgress.phase === "downloading" || updateProgress.phase === "installing" ? "处理中..." : "下载并安装"}
                </button>
                <button
                  className="btn btn-secondary"
                  disabled={isCheckingUpdate}
                  onClick={handleSkipFoundVersion}
                >
                  跳过此版本
                </button>
                <button
                  className="btn btn-secondary"
                  disabled={isCheckingUpdate}
                  onClick={() => setAvailableUpdate(null)}
                >
                  收起更新详情
                </button>
              </div>
            </div>
          )}
          {updateMsg && (
            <div className="alert alert-info" style={{ fontSize: 13, alignSelf: "stretch" }}>
              {updateMsg}
            </div>
          )}
        </div>

      </div>

      {geminiEditDialog && (
        <div className="modal-overlay" onClick={() => setGeminiEditDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>设置 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={() => setGeminiEditDialog(null)}>✕</button>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div>
                <label className="form-label">额外启动参数</label>
                <input className="form-input" value={geminiEditDialog.extraArgs} onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, extraArgs: e.target.value })} />
              </div>
              <div>
                <label className="form-label">绑定账号 ID</label>
                <select
                  className="form-input"
                  value={geminiEditDialog.bindAccountId}
                  onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, bindAccountId: e.target.value })}
                  disabled={geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount}
                >
                  <option value="">未绑定账号</option>
                  {geminiAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {(account.label?.trim() || account.email)}{currentGeminiAccountId === account.id ? " (当前本地)" : ""}
                    </option>
                  ))}
                </select>
                <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                  {geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount
                    ? `当前会跟随本地 Gemini 账号：${getGeminiAccountLabel(currentGeminiAccountId)}`
                    : `当前选择：${getGeminiAccountLabel(geminiEditDialog.bindAccountId || null)}`}
                </div>
              </div>
              <div>
                <label className="form-label">项目 ID</label>
                <input className="form-input" value={geminiEditDialog.projectId} onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, projectId: e.target.value })} />
                <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                  {editSelectedGeminiProjectId
                    ? `实例会固定使用项目 ${editSelectedGeminiProjectId}`
                    : editAccountProjectId
                      ? `当前会沿用账号默认项目 ${editAccountProjectId}`
                      : "当前没有项目覆盖，会直接沿用本地 Gemini 默认行为"}
                </div>
              </div>
              {geminiEditDialog.instance.is_default && (
                <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={geminiEditDialog.followLocalAccount}
                    onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, followLocalAccount: e.target.checked })}
                  />
                  跟随当前本地 Gemini 账号
                </label>
              )}
              {geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount && !currentGeminiAccountId && (
                <div className="alert alert-info" style={{ fontSize: 13 }}>
                  当前没有解析到本地 Gemini 账号。若继续保持跟随模式，默认实例启动时不会有可注入的账号。
                </div>
              )}
              {!geminiEditDialog.instance.is_default || !geminiEditDialog.followLocalAccount ? (
                editEffectiveGeminiAccountId && currentGeminiAccountId && editEffectiveGeminiAccountId !== currentGeminiAccountId ? (
                  <div className="alert alert-info" style={{ fontSize: 13 }}>
                    这个实例启动时会把当前本地账号从 {getGeminiAccountLabel(currentGeminiAccountId)} 切换为 {getGeminiAccountLabel(editEffectiveGeminiAccountId)}。
                  </div>
                ) : null
              ) : null}
              {editSelectedGeminiProjectId && editAccountProjectId && editSelectedGeminiProjectId !== editAccountProjectId && (
                <div className="alert alert-info" style={{ fontSize: 13 }}>
                  实例项目 {editSelectedGeminiProjectId} 会覆盖账号默认项目 {editAccountProjectId}。
                </div>
              )}
              <div
                style={{
                  padding: "10px 12px",
                  borderRadius: "var(--radius-sm)",
                  background: "rgba(255,255,255,0.04)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                }}
              >
                <div style={{ fontSize: 12, fontWeight: 600 }}>快捷修正建议</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  {geminiEditDialog.instance.is_default && currentGeminiAccountId && !geminiEditDialog.followLocalAccount && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ followLocalAccount: true, bindAccountId: "" })}
                    >
                      改为跟随当前本地账号
                    </button>
                  )}
                  {!geminiEditDialog.followLocalAccount && currentGeminiAccountId && geminiEditDialog.bindAccountId !== currentGeminiAccountId && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ bindAccountId: currentGeminiAccountId })}
                    >
                      改为绑定当前本地账号
                    </button>
                  )}
                  {!geminiEditDialog.followLocalAccount && !geminiEditDialog.bindAccountId && currentGeminiAccountId && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ bindAccountId: currentGeminiAccountId })}
                    >
                      绑定当前本地账号
                    </button>
                  )}
                  {editSelectedGeminiProjectId && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ projectId: "" })}
                    >
                      清除实例项目覆盖
                    </button>
                  )}
                  {!editSelectedGeminiProjectId && editAccountProjectId && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ projectId: editAccountProjectId })}
                    >
                      固定为账号默认项目
                    </button>
                  )}
                  {geminiEditDialog.followLocalAccount && !currentGeminiAccountId && geminiAccounts.length > 0 && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => applyGeminiEditPatch({ followLocalAccount: false, bindAccountId: geminiAccounts[0]?.id || "" })}
                    >
                      改为固定绑定账号
                    </button>
                  )}
                </div>
                <div className="text-muted" style={{ fontSize: 12 }}>
                  这些操作只会改当前弹层里的配置草稿，真正生效仍然要点保存。
                </div>
              </div>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setGeminiEditDialog(null)}>取消</button>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    try {
                      await api.geminiInstances.update(
                        geminiEditDialog.instance.id,
                        geminiEditDialog.extraArgs,
                        geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount
                          ? null
                          : geminiEditDialog.bindAccountId.trim() || null,
                        geminiEditDialog.projectId.trim() || null,
                        geminiEditDialog.instance.is_default ? geminiEditDialog.followLocalAccount : null,
                      );
                      setGeminiInstanceMsg("Gemini 实例设置已更新");
                      setGeminiEditDialog(null);
                      await reloadGeminiInstances();
                    } catch (e) {
                      setGeminiInstanceMsg(`更新 Gemini 实例失败: ${e}`);
                    }
                  }}
                >
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmDeleteGeminiId && (
        <div className="modal-overlay" onClick={() => setConfirmDeleteGeminiId(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>删除 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDeleteGeminiId(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>确认删除这个 Gemini 实例目录索引吗？不会删除真实文件。</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmDeleteGeminiId(null)}>取消</button>
                <button className="btn btn-danger" onClick={() => handleDeleteGeminiInstance(confirmDeleteGeminiId)}>删除</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmWebdavPull && (
        <div className="modal-overlay" onClick={() => !webdavLoading && setConfirmWebdavPull(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>确认 WebDAV Pull</h2>
              <button className="btn btn-icon" onClick={() => setConfirmWebdavPull(false)}>✕</button>
            </div>
            <div className="modal-body">
              <p>警告：拉取将会用云端配置覆盖本地配置（增量覆盖），确定要继续吗？</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmWebdavPull(false)} disabled={webdavLoading}>取消</button>
                <button className="btn btn-danger" onClick={handleWebdavPull} disabled={webdavLoading}>
                  {webdavLoading ? "拉取中..." : "确认拉取"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
