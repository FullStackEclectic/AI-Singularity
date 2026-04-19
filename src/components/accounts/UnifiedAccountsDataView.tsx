import type { RefObject } from "react";
import type { VirtualItem } from "@tanstack/react-virtual";
import { Box, RefreshCw } from "lucide-react";
import type { ApiKey, IdeAccount } from "../../types";
import type { AccountTagGroup, AccountViewMode } from "./accountViewUtils";
import { UnifiedAccountsGridCard } from "./UnifiedAccountsGridCard";
import { UnifiedAccountsVirtualRow } from "./UnifiedAccountsVirtualRow";
import type { AccountRenderRow, UnifiedAccountItem } from "./unifiedAccountsTypes";
import "./UnifiedAccountsDataView.css";

export function UnifiedAccountsDataView({
  accountViewMode,
  groupByTag,
  privacy,
  parentRef,
  isLoading,
  displayItemsLength,
  emptyStateMessage,
  currentProblemViewLabel,
  accountGroupFilter,
  searchQuery,
  onClearProblemFilters,
  onClearAccountGroupFilter,
  onClearSearch,
  gridSections,
  selectedIdeIds,
  onToggleIdeSelected,
  isStatusActionPending,
  currentIdeAccountIds,
  accountGroupByAccountId,
  getItemDisplayName,
  getItemTimeSummary,
  onCreateShareToken,
  onRunDailyCheckin,
  onRefreshApiBalance,
  onCheckApiKey,
  onDeleteApiKey,
  onSetCurrentIdeAccount,
  onRefreshIdeAccount,
  onOpenGeminiProject,
  onOpenCodexApiKey,
  onOpenIdeLabel,
  onDeleteIdeAccount,
  groupedRenderRows,
  totalVirtualSize,
  virtualRows,
}: {
  accountViewMode: AccountViewMode;
  groupByTag: boolean;
  privacy: boolean;
  parentRef: RefObject<HTMLDivElement | null>;
  isLoading: boolean;
  displayItemsLength: number;
  emptyStateMessage: string;
  currentProblemViewLabel: string | null;
  accountGroupFilter: string;
  searchQuery: string;
  onClearProblemFilters: () => void;
  onClearAccountGroupFilter: () => void;
  onClearSearch: () => void;
  gridSections: AccountTagGroup<UnifiedAccountItem>[];
  selectedIdeIds: string[];
  onToggleIdeSelected: (id: string) => void;
  isStatusActionPending: boolean;
  currentIdeAccountIds: Record<string, string | null>;
  accountGroupByAccountId: Map<string, { name: string }>;
  getItemDisplayName: (item: UnifiedAccountItem) => string;
  getItemTimeSummary: (item: UnifiedAccountItem) => string;
  onCreateShareToken: (item: UnifiedAccountItem) => void;
  onRunDailyCheckin: (account: IdeAccount) => void;
  onRefreshApiBalance: (id: string) => void;
  onCheckApiKey: (id: string) => void;
  onDeleteApiKey: (key: ApiKey) => void;
  onSetCurrentIdeAccount: (account: IdeAccount) => void;
  onRefreshIdeAccount: (account: IdeAccount) => void;
  onOpenGeminiProject: (account: IdeAccount) => void;
  onOpenCodexApiKey: (account: IdeAccount) => void;
  onOpenIdeLabel: (account: IdeAccount) => void;
  onDeleteIdeAccount: (account: IdeAccount) => void;
  groupedRenderRows: AccountRenderRow[];
  totalVirtualSize: number;
  virtualRows: VirtualItem[];
}) {
  return (
    <div
      className={`table-container ${accountViewMode === "grid" ? "grid-mode" : accountViewMode === "compact" ? "compact-mode" : ""}`}
      ref={parentRef}
    >
      {accountViewMode !== "grid" && (
        <div className={`data-table-header ${accountViewMode === "compact" ? "compact" : ""}`}>
          <div className="col-id">标识符 (UID)</div>
          <div className="col-platform">渠道类型</div>
          <div className="col-status">状态</div>
          <div className="col-balance">剩余额度</div>
          <div className="col-time">最后使用/心跳</div>
          <div className="col-actions">高危操作</div>
        </div>
      )}

      {isLoading ? (
        <div className="empty-state">
          <RefreshCw size={24} className="spin" />
          <span>核心数据网络拉取中...</span>
        </div>
      ) : displayItemsLength === 0 ? (
        <div className="empty-state">
          <Box size={32} opacity={0.5} />
          <span>{emptyStateMessage}</span>
          {currentProblemViewLabel && (
            <button className="btn-outline" onClick={onClearProblemFilters}>
              清除问题筛选
            </button>
          )}
          {accountGroupFilter !== "all" && (
            <button className="btn-outline" onClick={onClearAccountGroupFilter}>
              清除分组筛选
            </button>
          )}
          {searchQuery.trim() && (
            <button className="btn-outline" onClick={onClearSearch}>
              清除搜索
            </button>
          )}
        </div>
      ) : accountViewMode === "grid" ? (
        <div className="accounts-grid-board">
          {gridSections.map((section) => (
            <section key={section.key} className="accounts-grid-section">
              {groupByTag && (
                <div className="accounts-grid-section-header">
                  <span>{section.label}</span>
                  <span>{section.items.length} 个账号</span>
                </div>
              )}
              <div className="accounts-grid-list">
                {section.items.map((item, index) => (
                  <UnifiedAccountsGridCard
                    key={`${item.type}:${item.data.id}:${index}`}
                    item={item}
                    itemKey={`${item.type}:${item.data.id}:${index}`}
                    privacy={privacy}
                    selectedIdeIds={selectedIdeIds}
                    onToggleIdeSelected={onToggleIdeSelected}
                    isStatusActionPending={isStatusActionPending}
                    currentIdeAccountIds={currentIdeAccountIds}
                    accountGroupByAccountId={accountGroupByAccountId}
                    getItemDisplayName={getItemDisplayName}
                    getItemTimeSummary={getItemTimeSummary}
                    onCreateShareToken={onCreateShareToken}
                    onRunDailyCheckin={onRunDailyCheckin}
                    onRefreshApiBalance={onRefreshApiBalance}
                    onCheckApiKey={onCheckApiKey}
                    onSetCurrentIdeAccount={onSetCurrentIdeAccount}
                    onRefreshIdeAccount={onRefreshIdeAccount}
                    onOpenGeminiProject={onOpenGeminiProject}
                    onOpenCodexApiKey={onOpenCodexApiKey}
                    onOpenIdeLabel={onOpenIdeLabel}
                    onDeleteIdeAccount={onDeleteIdeAccount}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      ) : (
        <div className="virtual-list-inner" style={{ height: `${totalVirtualSize}px` }}>
          {virtualRows.map((virtualRow) => {
            const row = groupedRenderRows[virtualRow.index];
            if (!row) return null;
            if (row.type === "group") {
              return (
                <div
                  key={row.key}
                  className="data-group-row"
                  data-index={virtualRow.index}
                  style={{
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <span>{row.label}</span>
                  <span>{row.count} 个账号</span>
                </div>
              );
            }
            const item = row.item;
            return (
              <UnifiedAccountsVirtualRow
                key={row.key}
                item={item}
                rowKey={row.key}
                rowIndex={virtualRow.index}
                rowSize={virtualRow.size}
                rowStart={virtualRow.start}
                compact={accountViewMode === "compact"}
                privacy={privacy}
                selectedIdeIds={selectedIdeIds}
                onToggleIdeSelected={onToggleIdeSelected}
                isStatusActionPending={isStatusActionPending}
                currentIdeAccountIds={currentIdeAccountIds}
                accountGroupByAccountId={accountGroupByAccountId}
                getItemDisplayName={getItemDisplayName}
                getItemTimeSummary={getItemTimeSummary}
                onCreateShareToken={onCreateShareToken}
                onRunDailyCheckin={onRunDailyCheckin}
                onRefreshApiBalance={onRefreshApiBalance}
                onCheckApiKey={onCheckApiKey}
                onDeleteApiKey={onDeleteApiKey}
                onSetCurrentIdeAccount={onSetCurrentIdeAccount}
                onRefreshIdeAccount={onRefreshIdeAccount}
                onOpenGeminiProject={onOpenGeminiProject}
                onOpenCodexApiKey={onOpenCodexApiKey}
                onOpenIdeLabel={onOpenIdeLabel}
                onDeleteIdeAccount={onDeleteIdeAccount}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
