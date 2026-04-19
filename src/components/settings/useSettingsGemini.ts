import type { Dispatch, SetStateAction } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, type CurrentAccountSnapshot, type GeminiInstanceRecord } from "../../lib/api";
import type { IdeAccount } from "../../types";
import type {
  GeminiEditDialogState,
  GeminiInstanceWarning,
  GeminiQuickUpdatePatch,
} from "./settingsTypes";

type UseSettingsGeminiParams = {
  ideAccounts: IdeAccount[];
  currentGeminiAccountId: string | null;
  geminiInstances: GeminiInstanceRecord[];
  defaultGeminiInstance: GeminiInstanceRecord | null;
  geminiInstanceName: string;
  geminiInstanceDir: string;
  geminiEditDialog: GeminiEditDialogState | null;
  setIdeAccounts: Dispatch<SetStateAction<IdeAccount[]>>;
  setCurrentSnapshots: Dispatch<SetStateAction<CurrentAccountSnapshot[]>>;
  setCurrentGeminiAccountId: Dispatch<SetStateAction<string | null>>;
  setGeminiInstances: Dispatch<SetStateAction<GeminiInstanceRecord[]>>;
  setDefaultGeminiInstance: Dispatch<SetStateAction<GeminiInstanceRecord | null>>;
  setGeminiInstanceName: Dispatch<SetStateAction<string>>;
  setGeminiInstanceDir: Dispatch<SetStateAction<string>>;
  setGeminiInstanceMsg: Dispatch<SetStateAction<string>>;
  setGeminiInstanceLoading: Dispatch<SetStateAction<boolean>>;
  setGeminiRefreshLoading: Dispatch<SetStateAction<boolean>>;
  setGeminiEditDialog: Dispatch<SetStateAction<GeminiEditDialogState | null>>;
  setConfirmDeleteGeminiId: Dispatch<SetStateAction<string | null>>;
};

export function useSettingsGemini({
  ideAccounts,
  currentGeminiAccountId,
  geminiInstances,
  defaultGeminiInstance,
  geminiInstanceName,
  geminiInstanceDir,
  geminiEditDialog,
  setIdeAccounts,
  setCurrentSnapshots,
  setCurrentGeminiAccountId,
  setGeminiInstances,
  setDefaultGeminiInstance,
  setGeminiInstanceName,
  setGeminiInstanceDir,
  setGeminiInstanceMsg,
  setGeminiInstanceLoading,
  setGeminiRefreshLoading,
  setGeminiEditDialog,
  setConfirmDeleteGeminiId,
}: UseSettingsGeminiParams) {
  const reloadGeminiInstances = async () => {
    const [instanceList, defaultInstance, snapshots, accounts] = await Promise.all([
      api.geminiInstances.list(),
      api.geminiInstances.getDefault(),
      api.providerCurrent.listSnapshots().catch(() => []),
      api.ideAccounts.list(),
    ]);
    setIdeAccounts(accounts);
    setCurrentSnapshots(snapshots);
    setCurrentGeminiAccountId(
      snapshots.find((item) => item.platform === "gemini")?.account_id ?? null
    );
    setGeminiInstances(instanceList);
    setDefaultGeminiInstance(defaultInstance);
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
  const getGeminiAccountProjectLabel = (accountId?: string | null) => {
    const matched = getGeminiAccount(accountId);
    return matched?.project_id?.trim() || null;
  };
  const getEffectiveGeminiAccountId = (instance: GeminiInstanceRecord) =>
    instance.is_default && instance.follow_local_account
      ? currentGeminiAccountId
      : instance.bind_account_id || null;
  const getEffectiveGeminiProjectId = (instance: GeminiInstanceRecord) =>
    instance.project_id?.trim() ||
    getGeminiAccountProjectLabel(getEffectiveGeminiAccountId(instance));
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
    return (
      !!effectiveAccountId &&
      !!currentGeminiAccountId &&
      effectiveAccountId !== currentGeminiAccountId
    );
  }).length;
  const geminiProjectOverrideCount = allVisibleGeminiInstances.filter(
    (instance) => !!instance.project_id?.trim()
  ).length;
  const geminiUninitializedCount = allVisibleGeminiInstances.filter(
    (instance) => !instance.initialized
  ).length;
  const geminiUnboundCount = allVisibleGeminiInstances.filter(
    (instance) => !getEffectiveGeminiAccountId(instance)
  ).length;

  const getGeminiInstanceWarnings = (
    instance: GeminiInstanceRecord
  ): GeminiInstanceWarning[] => {
    const warnings: GeminiInstanceWarning[] = [];
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
    patch: GeminiQuickUpdatePatch,
    successMessage: string
  ) => {
    try {
      await api.geminiInstances.update(
        instance.id,
        patch.extraArgs ?? instance.extra_args ?? "",
        patch.bindAccountId !== undefined ? patch.bindAccountId : instance.bind_account_id ?? null,
        patch.projectId !== undefined ? patch.projectId : instance.project_id ?? null,
        instance.is_default
          ? patch.followLocalAccount !== undefined
            ? patch.followLocalAccount
            : instance.follow_local_account ?? false
          : null
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

  const applyGeminiEditPatch = (patch: Partial<GeminiEditDialogState>) => {
    setGeminiEditDialog((current) => (current ? { ...current, ...patch } : current));
  };

  const handleSaveGeminiEdit = async () => {
    if (!geminiEditDialog) return;

    try {
      await api.geminiInstances.update(
        geminiEditDialog.instance.id,
        geminiEditDialog.extraArgs,
        geminiEditDialog.instance.is_default && geminiEditDialog.followLocalAccount
          ? null
          : geminiEditDialog.bindAccountId.trim() || null,
        geminiEditDialog.projectId.trim() || null,
        geminiEditDialog.instance.is_default ? geminiEditDialog.followLocalAccount : null
      );
      setGeminiInstanceMsg("Gemini 实例设置已更新");
      setGeminiEditDialog(null);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`更新 Gemini 实例失败: ${e}`);
    }
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

  return {
    reloadGeminiInstances,
    geminiAccounts,
    getGeminiAccountLabel,
    isCurrentLocalGeminiAccount,
    getGeminiAccountProjectLabel,
    getEffectiveGeminiAccountId,
    getEffectiveGeminiProjectId,
    formatGeminiLaunchTime,
    sortedGeminiInstances,
    allVisibleGeminiInstances,
    geminiConflictCount,
    geminiProjectOverrideCount,
    geminiUninitializedCount,
    geminiUnboundCount,
    getGeminiInstanceWarnings,
    editEffectiveGeminiAccountId,
    editSelectedGeminiProjectId,
    editAccountProjectId,
    applyGeminiEditPatch,
    handlePickGeminiDir,
    handleAddGeminiInstance,
    handleUpdateGeminiInstance,
    handleQuickUpdateGeminiInstance,
    handleSaveGeminiEdit,
    handleCopyGeminiLaunchCommand,
    handleLaunchGeminiInstance,
    handleDeleteGeminiInstance,
    handleRefreshGeminiRuntime,
  };
}

export type SettingsGeminiState = ReturnType<typeof useSettingsGemini>;
