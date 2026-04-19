import type { AccountGroup } from "../../types";
import type { AttentionReasonFilter } from "./unifiedAccountsUtils";
import type { AccountViewMode } from "./accountViewUtils";
import type { ActionMessage, IdeOverviewSummary } from "./unifiedAccountsTypes";
import "./UnifiedAccountsControlPanel.css";
import {
  CalendarCheck,
  Edit2,
  Eye,
  EyeOff,
  Folder,
  MonitorPlay,
  Plus,
  RefreshCw,
  Search,
  Share,
  Trash2,
  X,
} from "lucide-react";

export function UnifiedAccountsControlPanel({
  activeChannelName,
  totalFilteredCount,
  currentProblemViewLabel,
  currentGroupViewLabel,
  privacy,
  onTogglePrivacy,
  canBatchRefreshActiveIde,
  activeIdePlatform,
  isBatchRefreshingActiveIde,
  onBatchRefreshActiveIde,
  canBatchDailyCheckin,
  isStatusActionPending,
  selectedIdeCount,
  filteredDailyCheckinCount,
  onBatchDailyCheckin,
  canExportIdeAccounts,
  onExportIdeAccounts,
  canBatchEditIdeTags,
  onOpenBatchIdeTags,
  canBatchGroupIde,
  onOpenBatchGroupDialog,
  onOpenGroupManageDialog,
  isCheckAllKeysPending,
  onCheckAllKeys,
  onOpenAddWizard,
  actionMessage,
  ideOverview,
  isGroupViewActive,
  filterScopeLabel,
  currentGroupActionLabel,
  accountGroupFilter,
  onSetAccountGroupFilter,
  groupFilterOptions,
  filteredIdeIdsLength,
  onSelectCurrentGroupIde,
  isBatchRefreshPending,
  onRefreshCurrentGroup,
  onTagCurrentGroup,
  canBatchSetCurrentForGroupView,
  onSetCurrentForGroupView,
  onDeleteCurrentGroup,
  showAttentionOnly,
  onToggleAttentionOnly,
  filteredAttentionIdeIdsLength,
  onSelectAttentionIde,
  attentionReasonFilter,
  filteredAttentionReasonIdeIdsLength,
  attentionReasonLabel,
  onSelectAttentionReasonIde,
  onClearProblemFilters,
  onToggleAttentionReason,
  canRefreshAttentionReason,
  onRefreshAttentionReason,
  onTagAttentionReason,
  selectedVisibleIdeIdsCount,
  selectedIdePlatforms,
  selectedCurrentCount,
  canBatchSetCurrent,
  onToggleAllVisibleIde,
  onBatchSetCurrentSelected,
  onBatchRefreshSelected,
  onDeleteSelected,
  onClearSelected,
  accountViewMode,
  onSetAccountViewMode,
  groupByTag,
  displayItemsLength,
  onToggleGroupByTag,
  searchQuery,
  onSearchQueryChange,
  onClearSearch,
}: {
  activeChannelName: string;
  totalFilteredCount: number;
  currentProblemViewLabel: string | null;
  currentGroupViewLabel: string | null;
  privacy: boolean;
  onTogglePrivacy: () => void;
  canBatchRefreshActiveIde: boolean;
  activeIdePlatform: string | null;
  isBatchRefreshingActiveIde: boolean;
  onBatchRefreshActiveIde: () => void;
  canBatchDailyCheckin: boolean;
  isStatusActionPending: boolean;
  selectedIdeCount: number;
  filteredDailyCheckinCount: number;
  onBatchDailyCheckin: () => void;
  canExportIdeAccounts: boolean;
  onExportIdeAccounts: () => void;
  canBatchEditIdeTags: boolean;
  onOpenBatchIdeTags: () => void;
  canBatchGroupIde: boolean;
  onOpenBatchGroupDialog: () => void;
  onOpenGroupManageDialog: () => void;
  isCheckAllKeysPending: boolean;
  onCheckAllKeys: () => void;
  onOpenAddWizard: () => void;
  actionMessage: ActionMessage | null;
  ideOverview: IdeOverviewSummary | null;
  isGroupViewActive: boolean;
  filterScopeLabel: string;
  currentGroupActionLabel: string;
  accountGroupFilter: string;
  onSetAccountGroupFilter: (value: string) => void;
  groupFilterOptions: AccountGroup[];
  filteredIdeIdsLength: number;
  onSelectCurrentGroupIde: () => void;
  isBatchRefreshPending: boolean;
  onRefreshCurrentGroup: () => void;
  onTagCurrentGroup: () => void;
  canBatchSetCurrentForGroupView: boolean;
  onSetCurrentForGroupView: () => void;
  onDeleteCurrentGroup: () => void;
  showAttentionOnly: boolean;
  onToggleAttentionOnly: () => void;
  filteredAttentionIdeIdsLength: number;
  onSelectAttentionIde: () => void;
  attentionReasonFilter: AttentionReasonFilter | null;
  filteredAttentionReasonIdeIdsLength: number;
  attentionReasonLabel: string | null;
  onSelectAttentionReasonIde: () => void;
  onClearProblemFilters: () => void;
  onToggleAttentionReason: (reason: AttentionReasonFilter) => void;
  canRefreshAttentionReason: boolean;
  onRefreshAttentionReason: () => void;
  onTagAttentionReason: () => void;
  selectedVisibleIdeIdsCount: number;
  selectedIdePlatforms: string[];
  selectedCurrentCount: number;
  canBatchSetCurrent: boolean;
  onToggleAllVisibleIde: () => void;
  onBatchSetCurrentSelected: () => void;
  onBatchRefreshSelected: () => void;
  onDeleteSelected: () => void;
  onClearSelected: () => void;
  accountViewMode: AccountViewMode;
  onSetAccountViewMode: (mode: AccountViewMode) => void;
  groupByTag: boolean;
  displayItemsLength: number;
  onToggleGroupByTag: () => void;
  searchQuery: string;
  onSearchQueryChange: (value: string) => void;
  onClearSearch: () => void;
}) {
  const renderAttentionReasonButton = (
    reason: AttentionReasonFilter,
    label: string,
    count: number,
  ) => {
    if (count <= 0) {
      return null;
    }

    return (
      <button
        className={`btn-outline ${attentionReasonFilter === reason ? "active" : ""}`}
        onClick={() => onToggleAttentionReason(reason)}
      >
        {label}
      </button>
    );
  };

  return (
    <>
      <div className="main-header">
        <div className="main-title-area">
          <h1 className="main-title">{activeChannelName}</h1>
          <p className="main-subtitle">
            已筛选 {totalFilteredCount} 条可用账单记录
            {currentProblemViewLabel ? ` · ${currentProblemViewLabel}` : ""}
            {currentGroupViewLabel ? ` · ${currentGroupViewLabel}` : ""}
          </p>
        </div>

        <div className="header-actions">
          <button className={`btn-icon-label ${privacy ? "active" : ""}`} onClick={onTogglePrivacy}>
            {privacy ? <EyeOff size={15} /> : <Eye size={15} />}
            {" "}
            {privacy ? "隐私开启" : "明文显示"}
          </button>
          {canBatchRefreshActiveIde && (
            <button
              className="btn-outline"
              onClick={onBatchRefreshActiveIde}
              disabled={isBatchRefreshingActiveIde}
            >
              <RefreshCw size={15} className={isBatchRefreshingActiveIde ? "spin" : ""} />
              {isBatchRefreshingActiveIde ? "批量刷新中" : `批量刷新 ${activeIdePlatform}`}
            </button>
          )}
          {canBatchDailyCheckin && (
            <button className="btn-outline" disabled={isStatusActionPending} onClick={onBatchDailyCheckin}>
              <CalendarCheck size={15} className={isStatusActionPending ? "spin" : ""} />
              {isStatusActionPending
                ? "签到处理中"
                : selectedIdeCount > 0
                  ? `签到已选 (${selectedIdeCount})`
                  : `一键签到 (${filteredDailyCheckinCount})`}
            </button>
          )}
          {canExportIdeAccounts && (
            <button className="btn-outline" onClick={onExportIdeAccounts}>
              <Share size={15} />
              {selectedIdeCount > 0 ? `导出已选 (${selectedIdeCount})` : "导出 IDE 账号"}
            </button>
          )}
          {canBatchEditIdeTags && (
            <button className="btn-outline" onClick={onOpenBatchIdeTags}>
              <Edit2 size={15} />
              {selectedIdeCount > 0 ? `批量标签 (${selectedIdeCount})` : "批量标签"}
            </button>
          )}
          {canBatchGroupIde && (
            <button className="btn-outline" onClick={onOpenBatchGroupDialog}>
              <Folder size={15} />
              {selectedIdeCount > 0 ? `批量分组 (${selectedIdeCount})` : "批量分组"}
            </button>
          )}
          <button className="btn-outline" onClick={onOpenGroupManageDialog}>
            <Folder size={15} />
            分组管理
          </button>
          <button className="btn-outline" onClick={onCheckAllKeys} disabled={isCheckAllKeysPending}>
            <RefreshCw size={15} className={isCheckAllKeysPending ? "spin" : ""} />
            {isCheckAllKeysPending ? "全量探测中" : "一键探测筛选键"}
          </button>
          <button className="btn-primary" onClick={onOpenAddWizard}>
            <Plus size={15} />
            {" "}
            添加资产
          </button>
        </div>
      </div>

      <div className="filter-bar">
        {actionMessage && (
          <div className={`accounts-action-bar ${actionMessage.tone ?? "info"}`}>
            {actionMessage.text}
          </div>
        )}
        {ideOverview && (
          <div className="accounts-overview-bar">
            <div className="accounts-overview-card">
              <div className="accounts-overview-label">{isGroupViewActive ? "分组 IDE 总数" : "IDE 总数"}</div>
              <div className="accounts-overview-value">{ideOverview.total}</div>
            </div>
            <div className="accounts-overview-card">
              <div className="accounts-overview-label">当前账号</div>
              <div className="accounts-overview-value">{ideOverview.currentCount}</div>
            </div>
            <div className="accounts-overview-card">
              <div className="accounts-overview-label">健康账号</div>
              <div className="accounts-overview-value">{ideOverview.activeCount}</div>
            </div>
            <div className="accounts-overview-card warning">
              <div className="accounts-overview-label">需关注</div>
              <div className="accounts-overview-value">{ideOverview.attentionCount}</div>
            </div>
            <div className="accounts-overview-card">
              <div className="accounts-overview-label">7 天内使用</div>
              <div className="accounts-overview-value">{ideOverview.recentUsedCount}</div>
            </div>
            <div className="accounts-overview-card">
              <div className="accounts-overview-label">带标签</div>
              <div className="accounts-overview-value">{ideOverview.taggedCount}</div>
            </div>
            <div className="accounts-overview-meta">
              <span>范围：{filterScopeLabel}</span>
              <span>平台：{ideOverview.platforms.join(" / ")}</span>
              <span>最近同步：{ideOverview.latestSyncAt}</span>
              <span>最近使用：{ideOverview.latestUsedAt}</span>
            </div>
            <div className="accounts-overview-reasons">
              {ideOverview.expiredCount > 0 && (
                <span className="accounts-overview-reason">过期 {ideOverview.expiredCount}</span>
              )}
              {ideOverview.forbiddenCount > 0 && (
                <span className="accounts-overview-reason">封禁 {ideOverview.forbiddenCount}</span>
              )}
              {ideOverview.rateLimitedCount > 0 && (
                <span className="accounts-overview-reason">限流 {ideOverview.rateLimitedCount}</span>
              )}
              {ideOverview.proxyDisabledCount > 0 && (
                <span className="accounts-overview-reason">代理禁用 {ideOverview.proxyDisabledCount}</span>
              )}
              {ideOverview.manuallyDisabledCount > 0 && (
                <span className="accounts-overview-reason">人工禁用 {ideOverview.manuallyDisabledCount}</span>
              )}
              {ideOverview.attentionCount === 0 && (
                <span className="accounts-overview-reason success">
                  {isGroupViewActive ? `${currentGroupActionLabel} 当前没有需关注项` : "当前频道没有需关注项"}
                </span>
              )}
            </div>
            <div className="accounts-overview-actions">
              <button
                className={`btn-outline ${accountGroupFilter === "all" ? "active" : ""}`}
                onClick={() => onSetAccountGroupFilter("all")}
              >
                全部分组
              </button>
              <button
                className={`btn-outline ${accountGroupFilter === "__ungrouped__" ? "active" : ""}`}
                onClick={() => onSetAccountGroupFilter("__ungrouped__")}
              >
                未分组
              </button>
              {groupFilterOptions.map((group) => (
                <button
                  key={group.id}
                  className={`btn-outline ${accountGroupFilter === group.id ? "active" : ""}`}
                  onClick={() => onSetAccountGroupFilter(group.id)}
                >
                  {group.name}
                </button>
              ))}
              {accountGroupFilter !== "all" && (
                <>
                  <button
                    className="btn-outline"
                    disabled={filteredIdeIdsLength === 0}
                    onClick={onSelectCurrentGroupIde}
                  >
                    一键只选当前分组
                  </button>
                  <button
                    className="btn-outline"
                    disabled={isBatchRefreshPending || filteredIdeIdsLength === 0}
                    onClick={onRefreshCurrentGroup}
                  >
                    <RefreshCw size={15} className={isBatchRefreshPending ? "spin" : ""} />
                    {isBatchRefreshPending ? "处理中..." : "刷新当前分组"}
                  </button>
                  <button
                    className="btn-outline"
                    disabled={filteredIdeIdsLength === 0}
                    onClick={onTagCurrentGroup}
                  >
                    <Edit2 size={15} />
                    打标当前分组
                  </button>
                  <button
                    className="btn-outline"
                    disabled={!canBatchSetCurrentForGroupView}
                    onClick={onSetCurrentForGroupView}
                  >
                    <MonitorPlay size={15} />
                    设为当前（分组）
                  </button>
                  <button
                    className="btn-outline"
                    disabled={filteredIdeIdsLength === 0}
                    onClick={onDeleteCurrentGroup}
                  >
                    <Trash2 size={15} />
                    删除当前分组
                  </button>
                </>
              )}
            </div>
            <div className="accounts-overview-actions">
              <button
                className={`btn-outline ${showAttentionOnly ? "active" : ""}`}
                onClick={onToggleAttentionOnly}
              >
                {showAttentionOnly ? "显示全部 IDE" : "只看需关注"}
              </button>
              <button
                className="btn-outline"
                disabled={filteredAttentionIdeIdsLength === 0}
                onClick={onSelectAttentionIde}
              >
                一键只选需关注
              </button>
              {attentionReasonFilter && (
                <button
                  className="btn-outline"
                  disabled={filteredAttentionReasonIdeIdsLength === 0}
                  onClick={onSelectAttentionReasonIde}
                >
                  一键只选{attentionReasonLabel}
                </button>
              )}
              {(showAttentionOnly || attentionReasonFilter) && (
                <button className="btn-outline" onClick={onClearProblemFilters}>
                  清除问题筛选
                </button>
              )}
              {showAttentionOnly && !attentionReasonFilter && (
                <span className="accounts-overview-hint">
                  {isGroupViewActive ? `${currentGroupActionLabel} 中仅显示需关注的 IDE 账号` : "当前仅显示需关注的 IDE 账号"}
                </span>
              )}
              {attentionReasonFilter && (
                <span className="accounts-overview-hint">
                  {isGroupViewActive
                    ? `${currentGroupActionLabel} 中仅显示：${attentionReasonLabel}`
                    : `当前仅显示：${attentionReasonLabel}`}
                </span>
              )}
            </div>
            <div className="accounts-overview-actions">
              {renderAttentionReasonButton("expired", "只看过期", ideOverview.expiredCount)}
              {renderAttentionReasonButton("forbidden", "只看封禁", ideOverview.forbiddenCount)}
              {renderAttentionReasonButton("rate_limited", "只看限流", ideOverview.rateLimitedCount)}
              {renderAttentionReasonButton("proxy_disabled", "只看代理禁用", ideOverview.proxyDisabledCount)}
              {renderAttentionReasonButton("manually_disabled", "只看人工禁用", ideOverview.manuallyDisabledCount)}
            </div>
            {attentionReasonFilter && (
              <div className="accounts-overview-actions emphasis">
                {canRefreshAttentionReason ? (
                  <button
                    className="btn-outline"
                    disabled={isBatchRefreshPending || filteredAttentionReasonIdeIdsLength === 0}
                    onClick={onRefreshAttentionReason}
                  >
                    <RefreshCw size={15} className={isBatchRefreshPending ? "spin" : ""} />
                    {isBatchRefreshPending ? "处理中..." : `刷新当前${attentionReasonLabel}`}
                  </button>
                ) : (
                  <button
                    className="btn-outline"
                    disabled={filteredAttentionReasonIdeIdsLength === 0}
                    onClick={onTagAttentionReason}
                  >
                    <Edit2 size={15} />
                    为当前{attentionReasonLabel}账号批量打标
                  </button>
                )}
                <span className="accounts-overview-hint">
                  {canRefreshAttentionReason
                    ? `${isGroupViewActive ? `${currentGroupActionLabel} 的` : ""}${attentionReasonLabel}账号通常可以先尝试批量刷新，再决定是否删除或重新设为当前。`
                    : `${isGroupViewActive ? `${currentGroupActionLabel} 的` : ""}${attentionReasonLabel}账号更适合先批量打标归档，再进一步人工处理。`}
                </span>
              </div>
            )}
          </div>
        )}
        {filteredIdeIdsLength > 0 && (
          <div className="accounts-selection-bar">
            <button className="btn-outline" onClick={onToggleAllVisibleIde}>
              {selectedVisibleIdeIdsCount === filteredIdeIdsLength && filteredIdeIdsLength > 0
                ? (isGroupViewActive ? "取消全选当前分组" : "取消全选当前 IDE")
                : (isGroupViewActive ? "全选当前分组" : "全选当前 IDE")}
            </button>
            {selectedIdeCount > 0 && (
              <>
                <span className="accounts-selection-text">
                  {isGroupViewActive
                    ? `${currentGroupActionLabel} 已选 ${selectedIdeCount} 个 IDE 账号`
                    : `已选 ${selectedIdeCount} 个 IDE 账号`}
                </span>
                <div className="accounts-selection-tags">
                  {selectedIdePlatforms.map((platform) => (
                    <span key={platform} className="accounts-selection-chip">{platform}</span>
                  ))}
                  {selectedCurrentCount > 0 && (
                    <span className="accounts-selection-chip current">当前 {selectedCurrentCount}</span>
                  )}
                </div>
                <button
                  className="btn-outline"
                  disabled={!canBatchSetCurrent}
                  onClick={onBatchSetCurrentSelected}
                >
                  <MonitorPlay size={15} />
                  设为当前
                </button>
                {!canBatchSetCurrent && (
                  <span className="accounts-selection-text">批量设为当前仅支持同一平台的已选 IDE</span>
                )}
                <button
                  className="btn-outline"
                  onClick={onBatchRefreshSelected}
                  disabled={isBatchRefreshPending}
                >
                  <RefreshCw size={15} className={isBatchRefreshPending ? "spin" : ""} />
                  {isBatchRefreshPending ? "处理中..." : "刷新已选"}
                </button>
                <button className="btn-outline" onClick={onDeleteSelected}>
                  <Trash2 size={15} />
                  删除已选
                </button>
                <button className="btn-outline" onClick={onClearSelected}>
                  清空已选
                </button>
              </>
            )}
          </div>
        )}
        <div className="accounts-view-controls">
          <span className="accounts-view-label">视图模式</span>
          <button
            className={`btn-outline ${accountViewMode === "list" ? "active" : ""}`}
            onClick={() => onSetAccountViewMode("list")}
          >
            列表
          </button>
          <button
            className={`btn-outline ${accountViewMode === "grid" ? "active" : ""}`}
            onClick={() => onSetAccountViewMode("grid")}
          >
            网格
          </button>
          <button
            className={`btn-outline ${accountViewMode === "compact" ? "active" : ""}`}
            onClick={() => onSetAccountViewMode("compact")}
          >
            紧凑
          </button>
          <button
            className={`btn-outline ${groupByTag ? "active" : ""}`}
            disabled={displayItemsLength === 0}
            onClick={onToggleGroupByTag}
          >
            {groupByTag ? "按标签分组中" : "按标签分组"}
          </button>
        </div>
        <div className="search-box">
          <Search size={14} className="search-icon" />
          <input
            className="search-input"
            placeholder={`在 ${activeChannelName} 中搜索 UID、名字或标签...`}
            value={searchQuery}
            onChange={(event) => onSearchQueryChange(event.target.value)}
          />
          {searchQuery && (
            <X
              size={14}
              className="search-clear"
              onClick={onClearSearch}
              style={{ cursor: "pointer" }}
            />
          )}
        </div>
      </div>
    </>
  );
}
