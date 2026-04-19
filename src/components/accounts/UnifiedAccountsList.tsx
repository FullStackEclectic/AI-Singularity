import { useQueryClient } from "@tanstack/react-query";
import { UnifiedAccountsDialogsContainer } from "./UnifiedAccountsDialogsContainer";
import { UnifiedAccountsSidebar } from "./UnifiedAccountsSidebar";
import { UnifiedAccountsWorkspace } from "./UnifiedAccountsWorkspace";
import { useUnifiedAccountsActions } from "./useUnifiedAccountsActions";
import { useUnifiedAccountsDerivedState } from "./useUnifiedAccountsDerivedState";
import { useUnifiedAccountsDialogs } from "./useUnifiedAccountsDialogs";
import { useUnifiedAccountsPageState } from "./useUnifiedAccountsPageState";
import { useUnifiedAccountsQueries } from "./useUnifiedAccountsQueries";
import { useUnifiedAccountsVirtualizer } from "./useUnifiedAccountsVirtualizer";
import "./UnifiedAccountsList.css";

export default function UnifiedAccountsList() {
  const qc = useQueryClient();
  const pageState = useUnifiedAccountsPageState();
  const queries = useUnifiedAccountsQueries();
  const derivedState = useUnifiedAccountsDerivedState({
    accountGroupFilter: pageState.accountGroupFilter,
    accountGroups: queries.accountGroups,
    activeChannelId: pageState.activeChannelId,
    attentionReasonFilter: pageState.attentionReasonFilter,
    balances: queries.balances,
    currentIdeAccountIds: queries.currentIdeAccountIds,
    groupByTag: pageState.groupByTag,
    ideAccs: queries.ideAccs,
    keys: queries.keys,
    searchQuery: pageState.searchQuery,
    selectedIdeIds: pageState.selectedIdeIds,
    showAttentionOnly: pageState.showAttentionOnly,
  });
  const virtualizer = useUnifiedAccountsVirtualizer({
    accountViewMode: pageState.accountViewMode,
    activeChannelId: pageState.activeChannelId,
    groupByTag: pageState.groupByTag,
    groupedRenderRows: derivedState.groupedRenderRows,
  });
  const dialogs = useUnifiedAccountsDialogs({
    activeChannelName: derivedState.activeChannelName,
    filteredIdeAccounts: derivedState.filteredIdeAccounts,
    filteredIdeIds: derivedState.filteredIdeIds,
    qc,
    selectedIdeCount: derivedState.selectedIdeCount,
    selectedVisibleIdeIds: derivedState.selectedVisibleIdeIds,
    setActionMessage: pageState.setActionMessage,
  });
  const actions = useUnifiedAccountsActions({
    activeChannelId: pageState.activeChannelId,
    activeChannelName: derivedState.activeChannelName,
    activeIdePlatform: derivedState.activeIdePlatform,
    attentionReasonFilter: pageState.attentionReasonFilter,
    attentionReasonLabel: derivedState.attentionReasonLabel,
    canBatchSetCurrent: derivedState.canBatchSetCurrent,
    canBatchSetCurrentForGroupView: derivedState.canBatchSetCurrentForGroupView,
    currentGroupActionLabel: derivedState.currentGroupActionLabel,
    currentIdeAccountIds: queries.currentIdeAccountIds,
    displayItems: derivedState.displayItems,
    filteredAttentionIdeIds: derivedState.filteredAttentionIdeIds,
    filteredAttentionReasonIdeIds: derivedState.filteredAttentionReasonIdeIds,
    filteredDailyCheckinIdeAccounts: derivedState.filteredDailyCheckinIdeAccounts,
    filteredIdeAccounts: derivedState.filteredIdeAccounts,
    filteredIdeIds: derivedState.filteredIdeIds,
    filteredIdePlatforms: derivedState.filteredIdePlatforms,
    openConfirmDialog: dialogs.openConfirmDialog,
    selectedIdeCount: derivedState.selectedIdeCount,
    selectedIdePlatforms: derivedState.selectedIdePlatforms,
    selectedVisibleIdeAccounts: derivedState.selectedVisibleIdeAccounts,
    selectedVisibleIdeIds: derivedState.selectedVisibleIdeIds,
    setActionMessage: pageState.setActionMessage,
    setAttentionReasonFilter: pageState.setAttentionReasonFilter,
    setBatchIdeTagsDialog: dialogs.setBatchIdeTagsDialog,
    setSelectedIdeIds: pageState.setSelectedIdeIds,
    setShowAttentionOnly: pageState.setShowAttentionOnly,
  });

  return (
    <div className="unified-accounts-page">
      <UnifiedAccountsSidebar
        activeChannelId={pageState.activeChannelId}
        channels={derivedState.channels}
        onSelectChannel={pageState.setActiveChannelId}
      />
      <UnifiedAccountsWorkspace
        pageState={pageState}
        queries={queries}
        derivedState={derivedState}
        dialogs={dialogs}
        actions={actions}
        virtualizer={virtualizer}
      />
      <UnifiedAccountsDialogsContainer dialogs={dialogs} queries={queries} />
    </div>
  );
}
