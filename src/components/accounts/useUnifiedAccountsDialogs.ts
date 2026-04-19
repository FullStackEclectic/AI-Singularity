import { useState, type Dispatch, type SetStateAction } from "react";
import type { QueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import type { AccountGroup, IdeAccount } from "../../types";
import {
  type BatchIdeTagsDialogState,
  type CodexApiKeyDialogState,
  type ConfirmDialogState,
  type GeminiProjectDialogState,
  type IdeLabelDialogState,
} from "./UnifiedAccountsModals";
import type { AccountGroupDialogState, ActionMessage } from "./unifiedAccountsTypes";
import { parseIdeMeta } from "./unifiedAccountsUtils";

type UseUnifiedAccountsDialogsParams = {
  activeChannelName: string;
  filteredIdeAccounts: IdeAccount[];
  filteredIdeIds: string[];
  qc: QueryClient;
  selectedIdeCount: number;
  selectedVisibleIdeIds: string[];
  setActionMessage: Dispatch<SetStateAction<ActionMessage | null>>;
};

export function useUnifiedAccountsDialogs({
  activeChannelName,
  filteredIdeAccounts,
  filteredIdeIds,
  qc,
  selectedIdeCount,
  selectedVisibleIdeIds,
  setActionMessage,
}: UseUnifiedAccountsDialogsParams) {
  const [showAddWizard, setShowAddWizard] = useState(false);
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null);
  const [confirmDialogBusy, setConfirmDialogBusy] = useState(false);
  const [geminiProjectDialog, setGeminiProjectDialog] = useState<GeminiProjectDialogState | null>(null);
  const [geminiProjectBusy, setGeminiProjectBusy] = useState(false);
  const [codexApiKeyDialog, setCodexApiKeyDialog] = useState<CodexApiKeyDialogState | null>(null);
  const [codexApiKeyBusy, setCodexApiKeyBusy] = useState(false);
  const [ideLabelDialog, setIdeLabelDialog] = useState<IdeLabelDialogState | null>(null);
  const [ideLabelBusy, setIdeLabelBusy] = useState(false);
  const [batchIdeTagsDialog, setBatchIdeTagsDialog] = useState<BatchIdeTagsDialogState | null>(null);
  const [batchIdeTagsBusy, setBatchIdeTagsBusy] = useState(false);
  const [accountGroupDialog, setAccountGroupDialog] = useState<AccountGroupDialogState | null>(null);
  const [accountGroupBusy, setAccountGroupBusy] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [renamingGroupId, setRenamingGroupId] = useState<string | null>(null);
  const [renamingGroupName, setRenamingGroupName] = useState("");

  const openConfirmDialog = (config: ConfirmDialogState) => {
    setConfirmDialog(config);
    setConfirmDialogBusy(false);
  };

  const handleAddWizardSuccess = () => {
    setShowAddWizard(false);
    qc.invalidateQueries({ queryKey: ["keys"] });
    qc.invalidateQueries({ queryKey: ["ideAccounts"] });
    qc.invalidateQueries({ queryKey: ["accountGroups"] });
  };

  const handleClearGeminiProject = async () => {
    if (!geminiProjectDialog) return;
    try {
      setGeminiProjectBusy(true);
      await api.ideAccounts.setGeminiProject(geminiProjectDialog.account.id, null);
      setActionMessage({ text: "已清除 Gemini 项目绑定", tone: "success" });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      setGeminiProjectDialog(null);
    } catch (e) {
      setActionMessage({ text: "清除 Gemini 项目失败: " + e, tone: "error" });
    } finally {
      setGeminiProjectBusy(false);
    }
  };

  const handleSaveGeminiProject = async () => {
    if (!geminiProjectDialog) return;
    try {
      setGeminiProjectBusy(true);
      const selectedProjectId = geminiProjectDialog.value.trim();
      await api.ideAccounts.setGeminiProject(
        geminiProjectDialog.account.id,
        selectedProjectId || null
      );
      setActionMessage({
        text: selectedProjectId ? `已绑定 Gemini 项目：${selectedProjectId}` : "已清除 Gemini 项目绑定",
        tone: "success",
      });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      setGeminiProjectDialog(null);
    } catch (e) {
      setActionMessage({ text: "设置 Gemini 项目失败: " + e, tone: "error" });
    } finally {
      setGeminiProjectBusy(false);
    }
  };

  const handleSaveCodexApiKey = async () => {
    if (!codexApiKeyDialog) return;
    try {
      setCodexApiKeyBusy(true);
      await api.ideAccounts.updateCodexApiKey(
        codexApiKeyDialog.account.id,
        codexApiKeyDialog.apiKey.trim(),
        codexApiKeyDialog.baseUrl.trim() || null
      );
      setActionMessage({ text: "Codex API Key 凭证已更新", tone: "success" });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      setCodexApiKeyDialog(null);
    } catch (e) {
      setActionMessage({ text: "更新 Codex API Key 失败: " + e, tone: "error" });
    } finally {
      setCodexApiKeyBusy(false);
    }
  };

  const handleSaveIdeLabel = async () => {
    if (!ideLabelDialog) return;
    try {
      setIdeLabelBusy(true);
      await api.ideAccounts.updateLabel(
        ideLabelDialog.account.id,
        ideLabelDialog.label.trim() || null
      );
      setActionMessage({ text: "账号备注名已更新", tone: "success" });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      setIdeLabelDialog(null);
    } catch (e) {
      setActionMessage({ text: "更新备注名失败: " + e, tone: "error" });
    } finally {
      setIdeLabelBusy(false);
    }
  };

  const handleSaveBatchIdeTags = async () => {
    if (!batchIdeTagsDialog) return;
    try {
      setBatchIdeTagsBusy(true);
      const tags = batchIdeTagsDialog.tagsText
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean);
      const updated = await api.ideAccounts.batchUpdateTags(batchIdeTagsDialog.ids, tags);
      setActionMessage({ text: `已批量更新 ${updated} 个 IDE 账号标签`, tone: "success" });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      setBatchIdeTagsDialog(null);
    } catch (e) {
      setActionMessage({ text: "批量更新 IDE 标签失败: " + e, tone: "error" });
    } finally {
      setBatchIdeTagsBusy(false);
    }
  };

  const handleCreateAccountGroup = async () => {
    try {
      setAccountGroupBusy(true);
      await api.ideAccounts.createGroup(newGroupName.trim());
      setNewGroupName("");
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
      setActionMessage({ text: "账号分组已创建", tone: "success" });
    } catch (e) {
      setActionMessage({ text: "创建账号分组失败: " + e, tone: "error" });
    } finally {
      setAccountGroupBusy(false);
    }
  };

  const handleAssignAccountsToGroup = async (group: AccountGroup) => {
    if (!accountGroupDialog) return;
    try {
      setAccountGroupBusy(true);
      await api.ideAccounts.assignToGroup(group.id, accountGroupDialog.ids);
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
      setActionMessage({ text: `已将 ${accountGroupDialog.count} 个账号加入分组「${group.name}」`, tone: "success" });
      setAccountGroupDialog(null);
    } catch (e) {
      setActionMessage({ text: "批量分组失败: " + e, tone: "error" });
    } finally {
      setAccountGroupBusy(false);
    }
  };

  const handleRemoveAccountsFromGroup = async (group: AccountGroup) => {
    if (!accountGroupDialog) return;
    try {
      setAccountGroupBusy(true);
      await api.ideAccounts.removeFromGroup(group.id, accountGroupDialog.ids);
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
      setActionMessage({ text: `已将 ${accountGroupDialog.count} 个账号从分组「${group.name}」移出`, tone: "success" });
      setAccountGroupDialog(null);
    } catch (e) {
      setActionMessage({ text: "移出分组失败: " + e, tone: "error" });
    } finally {
      setAccountGroupBusy(false);
    }
  };

  const handleStartRenamingGroup = (group: AccountGroup) => {
    setRenamingGroupId(group.id);
    setRenamingGroupName(group.name);
  };

  const handleCancelRenamingGroup = () => {
    setRenamingGroupId(null);
    setRenamingGroupName("");
  };

  const handleSaveRenamingGroup = async (groupId: string) => {
    try {
      setAccountGroupBusy(true);
      await api.ideAccounts.renameGroup(groupId, renamingGroupName.trim());
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
      setActionMessage({ text: "账号分组已重命名", tone: "success" });
      handleCancelRenamingGroup();
    } catch (e) {
      setActionMessage({ text: "重命名账号分组失败: " + e, tone: "error" });
    } finally {
      setAccountGroupBusy(false);
    }
  };

  const handleDeleteAccountGroup = async (group: AccountGroup) => {
    try {
      setAccountGroupBusy(true);
      await api.ideAccounts.deleteGroup(group.id);
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
      setActionMessage({ text: `分组「${group.name}」已删除`, tone: "success" });
    } catch (e) {
      setActionMessage({ text: "删除账号分组失败: " + e, tone: "error" });
    } finally {
      setAccountGroupBusy(false);
    }
  };

  const handleOpenBatchIdeTags = () => {
    const targets = selectedIdeCount > 0
      ? filteredIdeAccounts.filter((account) => selectedVisibleIdeIds.includes(account.id))
      : filteredIdeAccounts;
    const tagPool = [...new Set(targets.flatMap((account) => account.tags || []))];
    setBatchIdeTagsDialog({
      ids: targets.map((account) => account.id),
      tagsText: tagPool.join(", "),
      count: targets.length,
      channelLabel: activeChannelName,
    });
  };

  const handleOpenBatchGroupDialog = () => {
    const targets = selectedIdeCount > 0
      ? selectedVisibleIdeIds
      : filteredIdeIds;
    setAccountGroupDialog({
      mode: "assign",
      ids: targets,
      count: targets.length,
      channelLabel: activeChannelName,
    });
  };

  const handleOpenGroupManageDialog = () => {
    setAccountGroupDialog({
      mode: "manage",
      ids: [],
      count: 0,
      channelLabel: activeChannelName,
    });
  };

  const handleOpenGeminiProject = async (account: IdeAccount) => {
    try {
      const projects = await api.ideAccounts.listGeminiProjects(account.id);
      if (projects.length === 0) {
        setActionMessage({ text: "当前账号没有可选的 Gemini Cloud 项目", tone: "info" });
        return;
      }
      setGeminiProjectDialog({
        account,
        projects,
        value: account.project_id || projects[0]?.project_id || "",
      });
    } catch (e) {
      setActionMessage({ text: "设置 Gemini 项目失败: " + e, tone: "error" });
    }
  };

  const handleOpenCodexApiKey = (account: IdeAccount) => {
    const meta = parseIdeMeta(account.meta_json);
    setCodexApiKeyDialog({
      account,
      apiKey: typeof meta.openai_api_key === "string" ? meta.openai_api_key : "",
      baseUrl: typeof meta.api_base_url === "string" ? meta.api_base_url : "",
    });
  };

  const handleOpenIdeLabel = (account: IdeAccount) => {
    setIdeLabelDialog({
      account,
      label: account.label || "",
    });
  };

  return {
    accountGroupBusy,
    accountGroupDialog,
    batchIdeTagsBusy,
    batchIdeTagsDialog,
    codexApiKeyBusy,
    codexApiKeyDialog,
    confirmDialog,
    confirmDialogBusy,
    geminiProjectBusy,
    geminiProjectDialog,
    handleAddWizardSuccess,
    handleAssignAccountsToGroup,
    handleCancelRenamingGroup,
    handleClearGeminiProject,
    handleCreateAccountGroup,
    handleDeleteAccountGroup,
    handleOpenBatchGroupDialog,
    handleOpenBatchIdeTags,
    handleOpenCodexApiKey,
    handleOpenGeminiProject,
    handleOpenGroupManageDialog,
    handleOpenIdeLabel,
    handleRemoveAccountsFromGroup,
    handleSaveBatchIdeTags,
    handleSaveCodexApiKey,
    handleSaveGeminiProject,
    handleSaveIdeLabel,
    handleSaveRenamingGroup,
    handleStartRenamingGroup,
    ideLabelBusy,
    ideLabelDialog,
    newGroupName,
    openConfirmDialog,
    renamingGroupId,
    renamingGroupName,
    setAccountGroupDialog,
    setBatchIdeTagsDialog,
    setCodexApiKeyDialog,
    setConfirmDialog,
    setConfirmDialogBusy,
    setGeminiProjectDialog,
    setIdeLabelDialog,
    setNewGroupName,
    setRenamingGroupName,
    setShowAddWizard,
    showAddWizard,
  };
}

export type UnifiedAccountsDialogsState = ReturnType<typeof useUnifiedAccountsDialogs>;
