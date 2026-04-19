import { useMemo } from "react";
import { PLATFORM_LABELS } from "../../types";
import type { AccountGroup, ApiKey, Balance, IdeAccount } from "../../types";
import {
  type AttentionReasonFilter,
  getAttentionReasonLabel,
  isCurrentIdeAccount,
  isIdeMatchingAttentionReason,
  isIdeNeedsAttention,
} from "./unifiedAccountsUtils";
import { supportsDailyCheckin } from "./platformStatusActionUtils";
import {
  type AccountTagGroup,
  groupAccountsByTag,
} from "./accountViewUtils";
import type {
  AccountRenderRow,
  ChannelMeta,
  ChannelType,
  IdeOverviewSummary,
  UnifiedAccountItem,
} from "./unifiedAccountsTypes";

type UseUnifiedAccountsDerivedStateParams = {
  accountGroupFilter: string;
  accountGroups: AccountGroup[];
  activeChannelId: string;
  attentionReasonFilter: AttentionReasonFilter | null;
  balances: Balance[];
  currentIdeAccountIds: Record<string, string | null>;
  groupByTag: boolean;
  ideAccs: IdeAccount[];
  keys: ApiKey[];
  searchQuery: string;
  selectedIdeIds: string[];
  showAttentionOnly: boolean;
};

type ChannelsSummary = {
  all: ChannelMeta;
  apiChs: ChannelMeta[];
  ideChs: ChannelMeta[];
};

type IdeUnifiedItem = Extract<UnifiedAccountItem, { type: "ide" }>;

export function useUnifiedAccountsDerivedState({
  accountGroupFilter,
  accountGroups,
  activeChannelId,
  attentionReasonFilter,
  balances,
  currentIdeAccountIds,
  groupByTag,
  ideAccs,
  keys,
  searchQuery,
  selectedIdeIds,
  showAttentionOnly,
}: UseUnifiedAccountsDerivedStateParams) {
  const balanceMap = useMemo(
    () => Object.fromEntries(balances.map((balance) => [balance.key_id, balance])),
    [balances]
  );

  const channels = useMemo<ChannelsSummary>(() => {
    const apiMap = new Map<string, number>();
    const ideMap = new Map<string, number>();

    keys.forEach((key) => apiMap.set(key.platform, (apiMap.get(key.platform) || 0) + 1));
    ideAccs.forEach((account) => ideMap.set(account.origin_platform, (ideMap.get(account.origin_platform) || 0) + 1));

    const all: ChannelMeta = {
      id: "all",
      type: "all",
      label: "全部资产大盘",
      count: keys.length + ideAccs.length,
    };

    const apiChs: ChannelMeta[] = Array.from(apiMap.entries())
      .map(([platform, count]) => ({
        id: `api_${platform}`,
        type: "api" as ChannelType,
        count,
        label: PLATFORM_LABELS[platform as keyof typeof PLATFORM_LABELS] || platform,
      }))
      .sort((a, b) => b.count - a.count);

    const ideChs: ChannelMeta[] = Array.from(ideMap.entries())
      .map(([platform, count]) => ({
        id: `ide_${platform}`,
        type: "ide" as ChannelType,
        count,
        label: platform,
      }))
      .sort((a, b) => b.count - a.count);

    return { all, apiChs, ideChs };
  }, [ideAccs, keys]);

  const accountGroupByAccountId = useMemo(() => {
    const map = new Map<string, AccountGroup>();
    for (const group of accountGroups) {
      for (const accountId of group.account_ids || []) {
        map.set(accountId, group);
      }
    }
    return map;
  }, [accountGroups]);

  const displayItems = useMemo(() => {
    let rawItems: UnifiedAccountItem[] = [];

    if (activeChannelId === "all") {
      rawItems = [
        ...keys.map((key): UnifiedAccountItem => ({ type: "api", data: key, balance: balanceMap[key.id] })),
        ...ideAccs.map((account): UnifiedAccountItem => ({ type: "ide", data: account })),
      ];
    } else if (activeChannelId.startsWith("api_")) {
      const platform = activeChannelId.replace("api_", "");
      rawItems = keys
        .filter((key) => key.platform === platform)
        .map((key) => ({ type: "api", data: key, balance: balanceMap[key.id] }));
    } else if (activeChannelId.startsWith("ide_")) {
      const platform = activeChannelId.replace("ide_", "");
      rawItems = ideAccs
        .filter((account) => account.origin_platform === platform)
        .map((account) => ({ type: "ide", data: account }));
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      rawItems = rawItems.filter((item) => {
        if (item.type === "api") {
          return item.data.name.toLowerCase().includes(query) || item.data.platform.toLowerCase().includes(query);
        }
        return item.data.email.toLowerCase().includes(query) || item.data.origin_platform.toLowerCase().includes(query);
      });
    }

    if (showAttentionOnly || attentionReasonFilter) {
      rawItems = rawItems.filter(
        (item) =>
          item.type === "ide" &&
          (attentionReasonFilter
            ? isIdeMatchingAttentionReason(item.data, attentionReasonFilter)
            : isIdeNeedsAttention(item.data))
      );
    }

    if (accountGroupFilter !== "all") {
      rawItems = rawItems.filter((item) => {
        if (item.type !== "ide") return false;
        const group = accountGroupByAccountId.get(item.data.id);
        if (accountGroupFilter === "__ungrouped__") {
          return !group;
        }
        return group?.id === accountGroupFilter;
      });
    }

    return rawItems.sort((a, b) => {
      const getTime = (item: UnifiedAccountItem) =>
        item.type === "api"
          ? new Date(item.data.created_at).getTime()
          : new Date(item.data.last_used).getTime();
      return getTime(b) - getTime(a);
    });
  }, [
    accountGroupByAccountId,
    accountGroupFilter,
    activeChannelId,
    attentionReasonFilter,
    balanceMap,
    ideAccs,
    keys,
    searchQuery,
    showAttentionOnly,
  ]);

  const groupedTagSections = useMemo<AccountTagGroup<UnifiedAccountItem>[]>(() => {
    if (!groupByTag) return [];
    return groupAccountsByTag(displayItems);
  }, [displayItems, groupByTag]);

  const groupedRenderRows = useMemo<AccountRenderRow[]>(() => {
    if (!groupByTag) {
      return displayItems.map((item, index) => ({
        type: "item" as const,
        key: `${item.type}:${item.data.id}:${index}`,
        item,
      }));
    }

    const rows: AccountRenderRow[] = [];
    for (const group of groupedTagSections) {
      rows.push({
        type: "group",
        key: `group:${group.key}`,
        label: group.label,
        count: group.items.length,
      });
      group.items.forEach((item, index) => {
        rows.push({
          type: "item",
          key: `${item.type}:${item.data.id}:${group.key}:${index}`,
          item,
        });
      });
    }
    return rows;
  }, [displayItems, groupedTagSections, groupByTag]);

  const gridSections = useMemo<AccountTagGroup<UnifiedAccountItem>[]>(() => {
    if (groupByTag) return groupedTagSections;
    return [{ key: "__all__", label: "全部账号", items: displayItems }];
  }, [displayItems, groupedTagSections, groupByTag]);

  const activeChannelName = channels.all.id === activeChannelId
    ? "系统全局资产"
    : [...channels.apiChs, ...channels.ideChs].find((channel) => channel.id === activeChannelId)?.label || "未知渠道";
  const isIdeProblemViewAvailable = activeChannelId === "all" || activeChannelId.startsWith("ide_");

  const totalFilteredCount = displayItems.length;
  const filteredIdeItems = displayItems.filter((item): item is IdeUnifiedItem => item.type === "ide");
  const filteredIdeAccounts = filteredIdeItems.map((item) => item.data);
  const filteredIdeIds = filteredIdeItems.map((item) => item.data.id);
  const filteredDailyCheckinIdeItems = filteredIdeItems.filter((item) => supportsDailyCheckin(item.data));
  const filteredDailyCheckinIdeAccounts = filteredDailyCheckinIdeItems.map((item) => item.data);
  const filteredDailyCheckinIds = filteredDailyCheckinIdeItems.map((item) => item.data.id);
  const filteredAttentionIdeIds = filteredIdeItems
    .filter((item) => isIdeNeedsAttention(item.data))
    .map((item) => item.data.id);
  const filteredAttentionReasonIdeIds = attentionReasonFilter
    ? filteredIdeItems
        .filter((item) => isIdeMatchingAttentionReason(item.data, attentionReasonFilter))
        .map((item) => item.data.id)
    : [];
  const selectedVisibleIdeIds = selectedIdeIds.filter((id) => filteredIdeIds.includes(id));
  const selectedVisibleIdeItems = filteredIdeItems.filter((item) => selectedVisibleIdeIds.includes(item.data.id));
  const selectedVisibleIdeAccounts = selectedVisibleIdeItems.map((item) => item.data);
  const canExportIdeAccounts = filteredIdeItems.length > 0 && (activeChannelId === "all" || activeChannelId.startsWith("ide_"));
  const canBatchEditIdeTags = filteredIdeItems.length > 0;
  const canBatchDailyCheckin = filteredDailyCheckinIds.length > 0;
  const batchRefreshablePlatforms = ["gemini", "codex", "cursor", "windsurf", "kiro", "qoder", "trae", "codebuddy", "codebuddy_cn", "workbuddy", "zed"];
  const activeIdePlatform = activeChannelId.startsWith("ide_") ? activeChannelId.replace("ide_", "") : null;
  const canBatchRefreshActiveIde = !!activeIdePlatform && batchRefreshablePlatforms.includes(activeIdePlatform);
  const selectedIdeCount = selectedVisibleIdeIds.length;
  const canBatchGroupIde = filteredIdeItems.length > 0;
  const selectedIdePlatforms = [...new Set(selectedVisibleIdeItems.map((item) => item.data.origin_platform))];
  const filteredIdePlatforms = [...new Set(filteredIdeItems.map((item) => item.data.origin_platform))];
  const canBatchSetCurrent = selectedIdeCount > 0 && selectedIdePlatforms.length === 1;
  const canBatchSetCurrentForGroupView =
    accountGroupFilter !== "all" && filteredIdeIds.length > 0 && filteredIdePlatforms.length === 1;
  const selectedCurrentCount = selectedVisibleIdeItems.filter((item) => isCurrentIdeAccount(item.data, currentIdeAccountIds)).length;
  const attentionReasonLabel = attentionReasonFilter ? getAttentionReasonLabel(attentionReasonFilter) : null;
  const currentGroupFilterLabel = accountGroupFilter === "__ungrouped__"
    ? "未分组"
    : accountGroups.find((group) => group.id === accountGroupFilter)?.name || null;
  const canRefreshAttentionReason =
    attentionReasonFilter === "expired" ||
    attentionReasonFilter === "forbidden" ||
    attentionReasonFilter === "rate_limited";
  const currentProblemViewLabel = attentionReasonFilter
    ? `问题视图：${attentionReasonLabel}`
    : showAttentionOnly
      ? "问题视图：需关注"
      : null;
  const currentGroupViewLabel = accountGroupFilter !== "all" && currentGroupFilterLabel
    ? `分组视图：${currentGroupFilterLabel}`
    : null;
  const currentGroupActionLabel = currentGroupFilterLabel ? `当前分组「${currentGroupFilterLabel}」` : "当前分组";
  const isGroupViewActive = accountGroupFilter !== "all";
  const filterScopeLabel = [
    `频道 ${activeChannelName}`,
    currentGroupViewLabel ? currentGroupViewLabel.replace("分组视图：", "分组 ") : null,
    currentProblemViewLabel ? currentProblemViewLabel.replace("问题视图：", "问题 ") : null,
    searchQuery.trim() ? `搜索 “${searchQuery.trim()}”` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  const emptyStateMessage = currentProblemViewLabel && currentGroupViewLabel
    ? `当前没有匹配“${currentProblemViewLabel.replace("问题视图：", "")}”且属于“${currentGroupFilterLabel || "未分组"}”的账号`
    : currentProblemViewLabel
      ? `当前没有匹配“${currentProblemViewLabel.replace("问题视图：", "")}”的账号`
      : currentGroupViewLabel
        ? `${currentGroupActionLabel} 下没有匹配的账号`
        : searchQuery.trim()
          ? `当前没有匹配关键词“${searchQuery.trim()}”的账号`
          : "当前汇聚池为空或被筛选掉";

  const ideOverview = useMemo<IdeOverviewSummary | null>(() => {
    if (filteredIdeItems.length === 0) return null;

    const now = Date.now();
    const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
    const platforms = [...new Set(filteredIdeItems.map((item) => item.data.origin_platform))];
    const currentCount = filteredIdeItems.filter((item) => isCurrentIdeAccount(item.data, currentIdeAccountIds)).length;
    const activeCount = filteredIdeItems.filter((item) => item.data.status === "active").length;
    const attentionCount = filteredIdeItems.filter((item) =>
      item.data.status !== "active" || item.data.is_proxy_disabled || !!item.data.disabled_reason
    ).length;
    const expiredCount = filteredIdeItems.filter((item) => item.data.status === "expired").length;
    const forbiddenCount = filteredIdeItems.filter((item) => item.data.status === "forbidden").length;
    const rateLimitedCount = filteredIdeItems.filter((item) => item.data.status === "rate_limited").length;
    const proxyDisabledCount = filteredIdeItems.filter((item) => item.data.is_proxy_disabled).length;
    const manuallyDisabledCount = filteredIdeItems.filter((item) => !!item.data.disabled_reason).length;
    const taggedCount = filteredIdeItems.filter((item) => (item.data.tags?.length || 0) > 0).length;
    const recentUsedCount = filteredIdeItems.filter((item) => {
      const timestamp = new Date(item.data.last_used).getTime();
      return Number.isFinite(timestamp) && now - timestamp <= sevenDaysMs;
    }).length;

    const lastUsedTimes = filteredIdeItems
      .map((item) => new Date(item.data.last_used).getTime())
      .filter((value) => Number.isFinite(value) && value > 0);
    const lastSyncTimes = filteredIdeItems
      .map((item) => new Date(item.data.token?.updated_at || item.data.updated_at).getTime())
      .filter((value) => Number.isFinite(value) && value > 0);

    return {
      total: filteredIdeItems.length,
      platforms,
      currentCount,
      activeCount,
      attentionCount,
      expiredCount,
      forbiddenCount,
      rateLimitedCount,
      proxyDisabledCount,
      manuallyDisabledCount,
      taggedCount,
      recentUsedCount,
      latestUsedAt: lastUsedTimes.length ? new Date(Math.max(...lastUsedTimes)).toLocaleString() : "—",
      latestSyncAt: lastSyncTimes.length ? new Date(Math.max(...lastSyncTimes)).toLocaleString() : "—",
    };
  }, [currentIdeAccountIds, filteredIdeItems]);

  const groupFilterBaseIdeItems = useMemo(() => {
    let items = ideAccs;
    if (activeChannelId.startsWith("ide_")) {
      const platform = activeChannelId.replace("ide_", "");
      items = items.filter((item) => item.origin_platform === platform);
    } else if (activeChannelId.startsWith("api_")) {
      return [] as IdeAccount[];
    }
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      items = items.filter((item) =>
        item.email.toLowerCase().includes(query) ||
        item.origin_platform.toLowerCase().includes(query) ||
        (item.label || "").toLowerCase().includes(query)
      );
    }
    return items;
  }, [activeChannelId, ideAccs, searchQuery]);

  const groupFilterOptions = useMemo(() => {
    const usedGroupIds = new Set(
      groupFilterBaseIdeItems
        .map((item) => accountGroupByAccountId.get(item.id)?.id)
        .filter(Boolean)
    );
    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [accountGroupByAccountId, accountGroups, groupFilterBaseIdeItems]);

  return {
    accountGroupByAccountId,
    activeChannelName,
    activeIdePlatform,
    canBatchDailyCheckin,
    canBatchEditIdeTags,
    canBatchGroupIde,
    canBatchRefreshActiveIde,
    canBatchSetCurrent,
    canBatchSetCurrentForGroupView,
    canExportIdeAccounts,
    canRefreshAttentionReason,
    channels,
    currentGroupActionLabel,
    currentGroupFilterLabel,
    currentGroupViewLabel,
    currentProblemViewLabel,
    displayItems,
    emptyStateMessage,
    filteredAttentionIdeIds,
    filteredAttentionReasonIdeIds,
    filteredDailyCheckinIdeAccounts,
    filteredDailyCheckinIdeItems,
    filteredDailyCheckinIds,
    filteredIdeAccounts,
    filteredIdeIds,
    filteredIdeItems,
    filteredIdePlatforms,
    filterScopeLabel,
    gridSections,
    groupFilterOptions,
    groupedRenderRows,
    ideOverview,
    isGroupViewActive,
    isIdeProblemViewAvailable,
    selectedCurrentCount,
    selectedIdeCount,
    selectedIdePlatforms,
    selectedVisibleIdeAccounts,
    selectedVisibleIdeIds,
    selectedVisibleIdeItems,
    totalFilteredCount,
    attentionReasonLabel,
  };
}

export type UnifiedAccountsDerivedState = ReturnType<typeof useUnifiedAccountsDerivedState>;
