import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { Dispatch, SetStateAction } from "react";
import { api } from "../../lib/api";
import type { ApiKey, IdeAccount } from "../../types";
import { buildDailyCheckinFeedback, supportsDailyCheckin } from "./platformStatusActionUtils";
import type { BatchIdeTagsDialogState, ConfirmDialogState } from "./UnifiedAccountsModals";
import type { ActionMessage, UnifiedAccountItem } from "./unifiedAccountsTypes";
import {
  type AttentionReasonFilter,
  formatIdePlatformLabel,
  getAttentionReasonSuggestedTag,
  getCurrentActionLabel,
  getIdeRefreshFailureMessage,
  getIdeRefreshSuccessMessage,
  isCurrentIdeAccount,
  isIdeMatchingAttentionReason,
} from "./unifiedAccountsUtils";

type UseUnifiedAccountsActionsParams = {
  activeChannelId: string;
  activeChannelName: string;
  activeIdePlatform: string | null;
  attentionReasonFilter: AttentionReasonFilter | null;
  attentionReasonLabel: string | null;
  canBatchSetCurrent: boolean;
  canBatchSetCurrentForGroupView: boolean;
  currentGroupActionLabel: string;
  currentIdeAccountIds: Record<string, string | null>;
  displayItems: UnifiedAccountItem[];
  filteredAttentionIdeIds: string[];
  filteredAttentionReasonIdeIds: string[];
  filteredDailyCheckinIdeAccounts: IdeAccount[];
  filteredIdeAccounts: IdeAccount[];
  filteredIdeIds: string[];
  filteredIdePlatforms: string[];
  openConfirmDialog: (config: ConfirmDialogState) => void;
  selectedIdeCount: number;
  selectedIdePlatforms: string[];
  selectedVisibleIdeAccounts: IdeAccount[];
  selectedVisibleIdeIds: string[];
  setActionMessage: Dispatch<SetStateAction<ActionMessage | null>>;
  setAttentionReasonFilter: Dispatch<SetStateAction<AttentionReasonFilter | null>>;
  setBatchIdeTagsDialog: Dispatch<SetStateAction<BatchIdeTagsDialogState | null>>;
  setSelectedIdeIds: Dispatch<SetStateAction<string[]>>;
  setShowAttentionOnly: Dispatch<SetStateAction<boolean>>;
};

export function useUnifiedAccountsActions({
  activeChannelId,
  activeChannelName,
  activeIdePlatform,
  attentionReasonFilter,
  attentionReasonLabel,
  canBatchSetCurrent,
  canBatchSetCurrentForGroupView,
  currentGroupActionLabel,
  currentIdeAccountIds,
  displayItems,
  filteredAttentionIdeIds,
  filteredAttentionReasonIdeIds,
  filteredDailyCheckinIdeAccounts,
  filteredIdeAccounts,
  filteredIdeIds,
  filteredIdePlatforms,
  openConfirmDialog,
  selectedIdeCount,
  selectedIdePlatforms,
  selectedVisibleIdeAccounts,
  selectedVisibleIdeIds,
  setActionMessage,
  setAttentionReasonFilter,
  setBatchIdeTagsDialog,
  setSelectedIdeIds,
  setShowAttentionOnly,
}: UseUnifiedAccountsActionsParams) {
  const qc = useQueryClient();

  const deleteKeyMut = useMutation({
    mutationFn: api.keys.delete,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });
  const checkKeyMut = useMutation({
    mutationFn: api.keys.check,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });
  const refreshBalMut = useMutation({
    mutationFn: (id: string) => api.balance.refreshOne(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }),
  });
  const deleteIdeMut = useMutation({
    mutationFn: api.ideAccounts.delete,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
    },
  });
  const refreshIdeMut = useMutation({
    mutationFn: api.ideAccounts.refresh,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });
  const refreshAllIdeByPlatformMut = useMutation({
    mutationFn: api.ideAccounts.refreshAllByPlatform,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });
  const batchRefreshIdeMut = useMutation({
    mutationFn: api.ideAccounts.batchRefresh,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });
  const checkAllKeysMut = useMutation({
    mutationFn: async (list: ApiKey[]) => {
      for (const key of list) {
        await api.keys.check(key.id);
      }
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });
  const statusActionMut = useMutation({
    mutationFn: (payload: { id: string; action: string; retryFailedTimes?: number | null }) =>
      api.ideAccounts.runStatusAction(payload.id, payload.action, payload.retryFailedTimes ?? null),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });

  const toggleAllVisibleIde = () => {
    if (filteredIdeIds.length > 0 && selectedVisibleIdeIds.length === filteredIdeIds.length) {
      setSelectedIdeIds((prev) => prev.filter((id) => !filteredIdeIds.includes(id)));
      return;
    }
    setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredIdeIds])]);
  };

  const runDailyCheckinForAccount = async (account: IdeAccount, retryFailedTimes = 1) => {
    try {
      const result = await statusActionMut.mutateAsync({
        id: account.id,
        action: "daily_checkin",
        retryFailedTimes,
      });
      setActionMessage({
        text: buildDailyCheckinFeedback(account, result),
        tone: result.success ? "success" : "info",
      });
      return result;
    } catch (e) {
      setActionMessage({
        text: `${formatIdePlatformLabel(account)} 每日签到失败: ${e}`,
        tone: "error",
      });
      throw e;
    }
  };

  const handleBatchRefreshActiveIde = () => {
    if (!activeIdePlatform) return;
    refreshAllIdeByPlatformMut.mutate(activeIdePlatform, {
      onSuccess: (count) =>
        setActionMessage({
          text: `${activeIdePlatform} 批量刷新完成：成功 ${count} 个账号`,
          tone: "success",
        }),
      onError: (e) =>
        setActionMessage({
          text: `${activeIdePlatform} 批量刷新失败: ${e}`,
          tone: "error",
        }),
    });
  };

  const handleBatchDailyCheckin = async () => {
    const targets = (selectedIdeCount > 0 ? selectedVisibleIdeAccounts : filteredDailyCheckinIdeAccounts).filter(
      (item) => supportsDailyCheckin(item)
    );
    if (targets.length === 0) return;

    let successCount = 0;
    let skippedCount = 0;
    let failedCount = 0;

    for (const account of targets) {
      try {
        const result = await runDailyCheckinForAccount(account, 1);
        if (result.success) {
          successCount += 1;
        } else {
          skippedCount += 1;
        }
      } catch {
        failedCount += 1;
      }
    }

    setActionMessage({
      text: `每日签到已执行 ${targets.length} 个账号：成功 ${successCount}，未完成 ${skippedCount}，失败 ${failedCount}`,
      tone: failedCount > 0 ? "error" : "success",
    });
  };

  const handleExportIdeAccounts = async () => {
    try {
      const ids = selectedIdeCount > 0 ? selectedVisibleIdeIds : filteredIdeAccounts.map((item) => item.id);
      const json = await api.ideAccounts.export(ids);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      const exportTag = activeChannelId === "all" ? "ide-accounts" : `${activeChannelId.replace("ide_", "")}-accounts`;
      anchor.download = `${exportTag}-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
      setActionMessage({ text: `已导出 ${ids.length} 个 IDE 账号`, tone: "success" });
    } catch (e) {
      setActionMessage({ text: "导出 IDE 账号失败: " + e, tone: "error" });
    }
  };

  const handleCheckAllKeys = () => {
    const keys = displayItems.map((item) => item.data).filter((data) => "platform" in data) as ApiKey[];
    if (keys.length > 0) {
      checkAllKeysMut.mutate(keys);
    }
  };

  const handleSelectCurrentGroupIde = () => {
    setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredIdeIds])]);
  };

  const handleRefreshCurrentGroup = () => {
    batchRefreshIdeMut.mutate(filteredIdeIds, {
      onSuccess: (count) =>
        setActionMessage({
          text: `${currentGroupActionLabel} 已批量刷新 ${count} 个 IDE 账号`,
          tone: "success",
        }),
      onError: (e) =>
        setActionMessage({
          text: `${currentGroupActionLabel} 批量刷新失败: ${e}`,
          tone: "error",
        }),
    });
  };

  const handleTagCurrentGroup = () => {
    const tagPool = [...new Set(filteredIdeAccounts.flatMap((item) => item.tags || []))];
    setBatchIdeTagsDialog({
      ids: filteredIdeIds,
      tagsText: tagPool.join(", "),
      count: filteredIdeIds.length,
      channelLabel: `${activeChannelName} · ${currentGroupActionLabel}`,
    });
  };

  const handleSetCurrentForGroupView = () => {
    openConfirmDialog({
      title: "当前分组批量设为当前",
      description: canBatchSetCurrentForGroupView
        ? `确认依次将 ${currentGroupActionLabel} 的 ${filteredIdeIds.length} 个 ${filteredIdePlatforms[0]} 账号设为当前吗？最终当前账号会是最后一个。`
        : "当前分组批量设为当前只支持同一平台账号。",
      confirmLabel: "立即执行",
      action: async () => {
        for (const id of filteredIdeIds) {
          await api.ideAccounts.forceInject(id);
        }
        setActionMessage({
          text: `${currentGroupActionLabel} 已依次切换 ${filteredIdeIds.length} 个 ${filteredIdePlatforms[0]} 账号，最后一个已成为当前账号`,
          tone: "success",
        });
        qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
      },
    });
  };

  const handleDeleteCurrentGroup = () => {
    openConfirmDialog({
      title: "删除当前分组账号",
      description: `确认删除 ${currentGroupActionLabel} 下的 ${filteredIdeIds.length} 个 IDE 账号吗？此操作无法撤销。`,
      confirmLabel: "批量删除",
      tone: "danger",
      action: async () => {
        const count = await api.ideAccounts.batchDelete(filteredIdeIds);
        setSelectedIdeIds((prev) => prev.filter((id) => !filteredIdeIds.includes(id)));
        qc.invalidateQueries({ queryKey: ["ideAccounts"] });
        qc.invalidateQueries({ queryKey: ["accountGroups"] });
        setActionMessage({ text: `${currentGroupActionLabel} 已删除 ${count} 个 IDE 账号`, tone: "success" });
      },
    });
  };

  const handleToggleAttentionOnly = () => {
    setShowAttentionOnly((prev) => !prev);
    setAttentionReasonFilter(null);
  };

  const handleSelectAttentionIde = () => {
    setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredAttentionIdeIds])]);
  };

  const handleSelectAttentionReasonIde = () => {
    setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredAttentionReasonIdeIds])]);
  };

  const handleClearProblemFilters = () => {
    setShowAttentionOnly(false);
    setAttentionReasonFilter(null);
  };

  const handleToggleAttentionReason = (reason: AttentionReasonFilter) => {
    setAttentionReasonFilter((prev) => (prev === reason ? null : reason));
  };

  const handleRefreshAttentionReason = () => {
    batchRefreshIdeMut.mutate(filteredAttentionReasonIdeIds, {
      onSuccess: (count) =>
        setActionMessage({
          text: `已批量刷新 ${count} 个${attentionReasonLabel}账号`,
          tone: "success",
        }),
      onError: (e) =>
        setActionMessage({
          text: `批量刷新${attentionReasonLabel}账号失败: ${e}`,
          tone: "error",
        }),
    });
  };

  const handleTagAttentionReason = () => {
    if (!attentionReasonFilter) return;
    const targets = filteredIdeAccounts.filter((item) => isIdeMatchingAttentionReason(item, attentionReasonFilter));
    const tagPool = [...new Set(targets.flatMap((item) => item.tags || []))];
    const suggestion = getAttentionReasonSuggestedTag(attentionReasonFilter);
    const nextTags = [...new Set([...tagPool, suggestion])].join(", ");
    setBatchIdeTagsDialog({
      ids: targets.map((item) => item.id),
      tagsText: nextTags,
      count: targets.length,
      channelLabel: `${activeChannelName} · ${attentionReasonLabel}`,
    });
  };

  const handleBatchSetCurrentSelected = () => {
    openConfirmDialog({
      title: "批量设为当前",
      description: canBatchSetCurrent
        ? `确认依次将这 ${selectedIdeCount} 个 ${selectedIdePlatforms[0]} 账号设为当前吗？最终当前账号会是最后一个。`
        : "批量设为当前只支持同一平台的已选 IDE 账号。",
      confirmLabel: "立即执行",
      action: async () => {
        for (const id of selectedVisibleIdeIds) {
          await api.ideAccounts.forceInject(id);
        }
        setActionMessage({
          text: `已依次切换 ${selectedIdeCount} 个 ${selectedIdePlatforms[0]} 账号，最后一个已成为当前账号`,
          tone: "success",
        });
        qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
      },
    });
  };

  const handleBatchRefreshSelected = () => {
    batchRefreshIdeMut.mutate(selectedVisibleIdeIds, {
      onSuccess: (count) => setActionMessage({ text: `已批量刷新 ${count} 个已选 IDE 账号`, tone: "success" }),
      onError: (e) => setActionMessage({ text: "批量刷新已选 IDE 账号失败: " + e, tone: "error" }),
    });
  };

  const handleDeleteSelected = () => {
    openConfirmDialog({
      title: "批量删除已选 IDE 账号",
      description: `确认删除当前已选的 ${selectedIdeCount} 个 IDE 账号吗？此操作无法撤销。`,
      confirmLabel: "批量删除",
      tone: "danger",
      action: async () => {
        const count = await api.ideAccounts.batchDelete(selectedVisibleIdeIds);
        setSelectedIdeIds((prev) => prev.filter((id) => !selectedVisibleIdeIds.includes(id)));
        setActionMessage({ text: `已删除 ${count} 个已选 IDE 账号`, tone: "success" });
      },
    });
  };

  const handleClearSelected = () => {
    setSelectedIdeIds((prev) => prev.filter((id) => !selectedVisibleIdeIds.includes(id)));
  };

  const handleCreateShareToken = (item: UnifiedAccountItem) => {
    openConfirmDialog({
      title: "签发直连 Token",
      description: `为 ${item.type === "api" ? item.data.name : item.data.email} 单独签发一个透传 Token。`,
      confirmLabel: "立即签发",
      action: async () => {
        await api.userTokens.create({
          username: `[极速生成] ${item.type === "api" ? item.data.name : item.data.email}`,
          description: JSON.stringify({ desc: "单点直连专用", scope: "single", single_account: item.data.id }),
          expires_type: "never",
          expires_at: null,
          max_ips: 0,
          curfew_start: null,
          curfew_end: null,
        });
        setActionMessage({ text: "已生成底座专属直连 Token，请切换至【分享额度】页面查看。", tone: "success" });
      },
    });
  };

  const handleRefreshApiBalance = (id: string) => {
    refreshBalMut.mutate(id);
  };

  const handleCheckApiKey = (id: string) => {
    checkKeyMut.mutate(id);
  };

  const handleDeleteApiKey = (key: ApiKey) => {
    openConfirmDialog({
      title: "删除密钥",
      description: `确认删除密钥 ${key.name} 吗？此操作无法撤销。`,
      confirmLabel: "删除",
      tone: "danger",
      action: async () => {
        await deleteKeyMut.mutateAsync(key.id);
      },
    });
  };

  const handleSetCurrentIdeAccount = (account: IdeAccount) => {
    const isCurrent = isCurrentIdeAccount(account, currentIdeAccountIds);
    openConfirmDialog({
      title: getCurrentActionLabel(account),
      description: isCurrent ? `${account.email} 已经是当前账号。` : `确认将 ${account.email} 设为当前本地账号吗？`,
      confirmLabel: "立即切换",
      action: async () => {
        await api.ideAccounts.forceInject(account.id);
        qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
        setActionMessage({ text: `${getCurrentActionLabel(account)}成功`, tone: "success" });
      },
    });
  };

  const handleRefreshIdeAccount = (account: IdeAccount) => {
    refreshIdeMut.mutate(account.id, {
      onSuccess: () =>
        setActionMessage({
          text: getIdeRefreshSuccessMessage(account.origin_platform),
          tone: "success",
        }),
      onError: (e) =>
        setActionMessage({
          text: getIdeRefreshFailureMessage(account.origin_platform, e),
          tone: "error",
        }),
    });
  };

  const handleDeleteIdeAccount = (account: IdeAccount) => {
    openConfirmDialog({
      title: "删除指纹账号",
      description: `确认删除 ${account.email} 吗？此操作无法撤销。`,
      confirmLabel: "删除",
      tone: "danger",
      action: async () => {
        await deleteIdeMut.mutateAsync(account.id);
      },
    });
  };

  return {
    batchRefreshPending: batchRefreshIdeMut.isPending,
    checkAllKeysPending: checkAllKeysMut.isPending,
    handleBatchDailyCheckin,
    handleBatchRefreshActiveIde,
    handleBatchRefreshSelected,
    handleBatchSetCurrentSelected,
    handleCheckAllKeys,
    handleCheckApiKey,
    handleClearProblemFilters,
    handleClearSelected,
    handleCreateShareToken,
    handleDeleteApiKey,
    handleDeleteCurrentGroup,
    handleDeleteIdeAccount,
    handleDeleteSelected,
    handleExportIdeAccounts,
    handleRefreshApiBalance,
    handleRefreshAttentionReason,
    handleRefreshCurrentGroup,
    handleRefreshIdeAccount,
    handleSelectAttentionIde,
    handleSelectAttentionReasonIde,
    handleSelectCurrentGroupIde,
    handleSetCurrentForGroupView,
    handleSetCurrentIdeAccount,
    handleTagAttentionReason,
    handleTagCurrentGroup,
    handleToggleAttentionOnly,
    handleToggleAttentionReason,
    isBatchRefreshingActiveIde: refreshAllIdeByPlatformMut.isPending,
    isStatusActionPending: statusActionMut.isPending,
    runDailyCheckinForAccount,
    toggleAllVisibleIde,
  };
}

export type UnifiedAccountsActionsState = ReturnType<typeof useUnifiedAccountsActions>;
