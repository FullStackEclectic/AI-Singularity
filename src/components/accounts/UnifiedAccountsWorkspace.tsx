import { maskEmail, maskToken } from "../../lib/privacyMode";
import { UnifiedAccountsControlPanel } from "./UnifiedAccountsControlPanel";
import { UnifiedAccountsDataView } from "./UnifiedAccountsDataView";
import type { UnifiedAccountItem } from "./unifiedAccountsTypes";
import type { UnifiedAccountsActionsState } from "./useUnifiedAccountsActions";
import type { UnifiedAccountsDerivedState } from "./useUnifiedAccountsDerivedState";
import type { UnifiedAccountsDialogsState } from "./useUnifiedAccountsDialogs";
import type { UnifiedAccountsPageState } from "./useUnifiedAccountsPageState";
import type { UnifiedAccountsQueriesState } from "./useUnifiedAccountsQueries";
import type { UnifiedAccountsVirtualizerState } from "./useUnifiedAccountsVirtualizer";

type UnifiedAccountsWorkspaceProps = {
  pageState: UnifiedAccountsPageState;
  queries: UnifiedAccountsQueriesState;
  derivedState: UnifiedAccountsDerivedState;
  dialogs: UnifiedAccountsDialogsState;
  actions: UnifiedAccountsActionsState;
  virtualizer: UnifiedAccountsVirtualizerState;
};

function getItemDisplayName(item: UnifiedAccountItem, privacy: boolean) {
  if (privacy) {
    return item.type === "api" ? maskToken(item.data.key_preview) : maskEmail(item.data.email);
  }
  return item.type === "api" ? item.data.name : item.data.label?.trim() || item.data.email;
}

function getItemTimeSummary(item: UnifiedAccountItem) {
  if (item.type === "api") {
    return item.data.created_at ? new Date(item.data.created_at).toLocaleString() : "未知";
  }
  return item.data.last_used ? new Date(item.data.last_used).toLocaleString() : "从未调用";
}

export function UnifiedAccountsWorkspace({
  pageState,
  queries,
  derivedState,
  dialogs,
  actions,
  virtualizer,
}: UnifiedAccountsWorkspaceProps) {
  return (
    <div className="unified-main">
      <UnifiedAccountsControlPanel
        activeChannelName={derivedState.activeChannelName}
        totalFilteredCount={derivedState.totalFilteredCount}
        currentProblemViewLabel={derivedState.currentProblemViewLabel}
        currentGroupViewLabel={derivedState.currentGroupViewLabel}
        privacy={pageState.privacy}
        onTogglePrivacy={pageState.togglePrivacy}
        canBatchRefreshActiveIde={derivedState.canBatchRefreshActiveIde}
        activeIdePlatform={derivedState.activeIdePlatform}
        isBatchRefreshingActiveIde={actions.isBatchRefreshingActiveIde}
        onBatchRefreshActiveIde={actions.handleBatchRefreshActiveIde}
        canBatchDailyCheckin={derivedState.canBatchDailyCheckin}
        isStatusActionPending={actions.isStatusActionPending}
        selectedIdeCount={derivedState.selectedIdeCount}
        filteredDailyCheckinCount={derivedState.filteredDailyCheckinIds.length}
        onBatchDailyCheckin={() => void actions.handleBatchDailyCheckin()}
        canExportIdeAccounts={derivedState.canExportIdeAccounts}
        onExportIdeAccounts={() => void actions.handleExportIdeAccounts()}
        canBatchEditIdeTags={derivedState.canBatchEditIdeTags}
        onOpenBatchIdeTags={dialogs.handleOpenBatchIdeTags}
        canBatchGroupIde={derivedState.canBatchGroupIde}
        onOpenBatchGroupDialog={dialogs.handleOpenBatchGroupDialog}
        onOpenGroupManageDialog={dialogs.handleOpenGroupManageDialog}
        isCheckAllKeysPending={actions.checkAllKeysPending}
        onCheckAllKeys={actions.handleCheckAllKeys}
        onOpenAddWizard={() => dialogs.setShowAddWizard(true)}
        actionMessage={pageState.actionMessage}
        ideOverview={derivedState.ideOverview}
        isGroupViewActive={derivedState.isGroupViewActive}
        filterScopeLabel={derivedState.filterScopeLabel}
        currentGroupActionLabel={derivedState.currentGroupActionLabel}
        accountGroupFilter={pageState.accountGroupFilter}
        onSetAccountGroupFilter={pageState.setAccountGroupFilter}
        groupFilterOptions={derivedState.groupFilterOptions}
        filteredIdeIdsLength={derivedState.filteredIdeIds.length}
        onSelectCurrentGroupIde={actions.handleSelectCurrentGroupIde}
        isBatchRefreshPending={actions.batchRefreshPending}
        onRefreshCurrentGroup={actions.handleRefreshCurrentGroup}
        onTagCurrentGroup={actions.handleTagCurrentGroup}
        canBatchSetCurrentForGroupView={derivedState.canBatchSetCurrentForGroupView}
        onSetCurrentForGroupView={actions.handleSetCurrentForGroupView}
        onDeleteCurrentGroup={actions.handleDeleteCurrentGroup}
        showAttentionOnly={pageState.showAttentionOnly}
        onToggleAttentionOnly={actions.handleToggleAttentionOnly}
        filteredAttentionIdeIdsLength={derivedState.filteredAttentionIdeIds.length}
        onSelectAttentionIde={actions.handleSelectAttentionIde}
        attentionReasonFilter={pageState.attentionReasonFilter}
        filteredAttentionReasonIdeIdsLength={derivedState.filteredAttentionReasonIdeIds.length}
        attentionReasonLabel={derivedState.attentionReasonLabel}
        onSelectAttentionReasonIde={actions.handleSelectAttentionReasonIde}
        onClearProblemFilters={actions.handleClearProblemFilters}
        onToggleAttentionReason={actions.handleToggleAttentionReason}
        canRefreshAttentionReason={derivedState.canRefreshAttentionReason}
        onRefreshAttentionReason={actions.handleRefreshAttentionReason}
        onTagAttentionReason={actions.handleTagAttentionReason}
        selectedVisibleIdeIdsCount={derivedState.selectedVisibleIdeIds.length}
        selectedIdePlatforms={derivedState.selectedIdePlatforms}
        selectedCurrentCount={derivedState.selectedCurrentCount}
        canBatchSetCurrent={derivedState.canBatchSetCurrent}
        onToggleAllVisibleIde={actions.toggleAllVisibleIde}
        onBatchSetCurrentSelected={actions.handleBatchSetCurrentSelected}
        onBatchRefreshSelected={actions.handleBatchRefreshSelected}
        onDeleteSelected={actions.handleDeleteSelected}
        onClearSelected={actions.handleClearSelected}
        accountViewMode={pageState.accountViewMode}
        onSetAccountViewMode={pageState.setAccountViewMode}
        groupByTag={pageState.groupByTag}
        displayItemsLength={derivedState.displayItems.length}
        onToggleGroupByTag={() => pageState.setGroupByTag((prev) => !prev)}
        searchQuery={pageState.searchQuery}
        onSearchQueryChange={pageState.setSearchQuery}
        onClearSearch={() => pageState.setSearchQuery("")}
      />

      <UnifiedAccountsDataView
        accountViewMode={pageState.accountViewMode}
        groupByTag={pageState.groupByTag}
        privacy={pageState.privacy}
        parentRef={virtualizer.parentRef}
        isLoading={queries.isLoading}
        displayItemsLength={derivedState.displayItems.length}
        emptyStateMessage={derivedState.emptyStateMessage}
        currentProblemViewLabel={derivedState.currentProblemViewLabel}
        accountGroupFilter={pageState.accountGroupFilter}
        searchQuery={pageState.searchQuery}
        onClearProblemFilters={actions.handleClearProblemFilters}
        onClearAccountGroupFilter={() => pageState.setAccountGroupFilter("all")}
        onClearSearch={() => pageState.setSearchQuery("")}
        gridSections={derivedState.gridSections}
        selectedIdeIds={pageState.selectedIdeIds}
        onToggleIdeSelected={pageState.toggleIdeSelected}
        isStatusActionPending={actions.isStatusActionPending}
        currentIdeAccountIds={queries.currentIdeAccountIds}
        accountGroupByAccountId={derivedState.accountGroupByAccountId}
        getItemDisplayName={(item) => getItemDisplayName(item, pageState.privacy)}
        getItemTimeSummary={getItemTimeSummary}
        onCreateShareToken={actions.handleCreateShareToken}
        onRunDailyCheckin={(account) => void actions.runDailyCheckinForAccount(account, 1)}
        onRefreshApiBalance={actions.handleRefreshApiBalance}
        onCheckApiKey={actions.handleCheckApiKey}
        onDeleteApiKey={actions.handleDeleteApiKey}
        onSetCurrentIdeAccount={actions.handleSetCurrentIdeAccount}
        onRefreshIdeAccount={actions.handleRefreshIdeAccount}
        onOpenGeminiProject={(account) => void dialogs.handleOpenGeminiProject(account)}
        onOpenCodexApiKey={dialogs.handleOpenCodexApiKey}
        onOpenIdeLabel={dialogs.handleOpenIdeLabel}
        onDeleteIdeAccount={actions.handleDeleteIdeAccount}
        groupedRenderRows={derivedState.groupedRenderRows}
        totalVirtualSize={virtualizer.rowVirtualizer.getTotalSize()}
        virtualRows={virtualizer.rowVirtualizer.getVirtualItems()}
      />
    </div>
  );
}
