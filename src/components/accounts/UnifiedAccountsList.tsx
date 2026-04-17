import { useState, useMemo, useRef, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type CurrentAccountSnapshot } from "../../lib/api";
import { PLATFORM_LABELS } from "../../types";
import type { ApiKey, IdeAccount, Balance, AccountGroup } from "../../types";
import AddAccountWizard from "./AddAccountWizard";
import UnifiedAccountBalanceCell from "./UnifiedAccountBalanceCell";
import UnifiedAccountStatusCell from "./UnifiedAccountStatusCell";
import {
  BatchIdeTagsDialogModal,
  type BatchIdeTagsDialogState,
  CodexApiKeyDialogModal,
  type CodexApiKeyDialogState,
  ConfirmDialogModal,
  type ConfirmDialogState,
  GeminiProjectDialogModal,
  type GeminiProjectDialogState,
  IdeLabelDialogModal,
  type IdeLabelDialogState,
} from "./UnifiedAccountsModals";
import { isPrivacyMode, setPrivacyMode, maskEmail, maskToken } from "../../lib/privacyMode";
import { buildDailyCheckinFeedback, supportsDailyCheckin } from "./platformStatusActionUtils";
import {
  type AttentionReasonFilter,
  formatCodebuddySummary,
  formatCodexApiKeyTooltip,
  formatCodexQuotaSummary,
  formatCursorSummary,
  formatGeminiQuotaSummary,
  formatIdePlatformLabel,
  formatKiroSummary,
  formatQoderSummary,
  formatTraeSummary,
  formatWindsurfSummary,
  getAttentionReasonLabel,
  getAttentionReasonSuggestedTag,
  getCurrentActionLabel,
  getIdeRefreshActionLabel,
  getIdeRefreshFailureMessage,
  getIdeRefreshSuccessMessage,
  isIdeRefreshSupported,
  isCodexApiKeyAccount,
  isCurrentIdeAccount,
  isIdeMatchingAttentionReason,
  isIdeNeedsAttention,
  parseIdeMeta,
} from "./unifiedAccountsUtils";
import {
  type AccountTagGroup,
  type AccountViewMode,
  groupAccountsByTag,
  persistAccountViewMode,
  persistTagGrouping,
  readPersistedAccountViewMode,
  readPersistedTagGrouping,
} from "./accountViewUtils";
import { 
  Database, Server, ShieldCheck, Box, 
  Search, Eye, EyeOff, RefreshCw, Plus, X, MonitorPlay, Share, Folder, Key, Edit2, Trash2, CalendarCheck
} from "lucide-react";
import "./UnifiedAccountsList.css";

// ─── 类型与辅助 ──────────────────────────────────────────────────────────

type ChannelType = "api" | "ide" | "all";
interface ChannelMeta {
  id: string;      // 例如 "all", "api_open_ai", "ide_vscode"
  type: ChannelType;
  label: string;
  count: number;
}

type ActionMessage = {
  text: string;
  tone?: "success" | "error" | "info";
};

type AccountGroupDialogState = {
  mode: "manage" | "assign";
  ids: string[];
  count: number;
  channelLabel: string;
};

type UnifiedAccountItem = 
  | { type: "api"; data: ApiKey; balance?: Balance }
  | { type: "ide"; data: IdeAccount };

type AccountRenderRow =
  | { type: "group"; key: string; label: string; count: number }
  | { type: "item"; key: string; item: UnifiedAccountItem };

// ─── 数据视图呈现组件 ────────────────────────────────────────────────────

export default function UnifiedAccountsList() {
  const qc = useQueryClient();
  const parentRef = useRef<HTMLDivElement>(null);

  // ---------- UI 状态 ----------
  const [showAddWizard, setShowAddWizard] = useState(false);
  const [privacy, setPrivacy] = useState(isPrivacyMode);
  const [actionMessage, setActionMessage] = useState<ActionMessage | null>(null);
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
  const [selectedIdeIds, setSelectedIdeIds] = useState<string[]>([]);
  const [showAttentionOnly, setShowAttentionOnly] = useState(false);
  const [attentionReasonFilter, setAttentionReasonFilter] = useState<AttentionReasonFilter | null>(null);
  const [accountGroupFilter, setAccountGroupFilter] = useState<string>("all");
  const [accountViewMode, setAccountViewMode] = useState<AccountViewMode>(() => readPersistedAccountViewMode());
  const [groupByTag, setGroupByTag] = useState<boolean>(() => readPersistedTagGrouping());

  // ---------- 过滤与侧边栏状态 ----------
  const [searchQuery, setSearchQuery] = useState("");
  const [activeChannelId, setActiveChannelId] = useState<string>("all");

  // ---------- Server 数据加载 ----------
  const { data: rawKeys = [], isLoading: keysLoading } = useQuery({ queryKey: ["keys"], queryFn: api.keys.list });
  // 过滤掉由终端配置同步模块自动托管的 "(Auto Key)"，不污染全局商业/核心资产池
  const keys = useMemo(() => rawKeys.filter((k) => !k.name.endsWith("(Auto Key)")), [rawKeys]);
  
  const { data: balances = [] } = useQuery({ queryKey: ["balances"], queryFn: api.balance.listAll, staleTime: 1000 * 60 * 5 });
  const balanceMap = Object.fromEntries(balances.map((b) => [b.key_id, b]));
  const { data: ideAccs = [], isLoading: ideLoading } = useQuery({ queryKey: ["ideAccounts"], queryFn: api.ideAccounts.list });
  const { data: accountGroups = [] } = useQuery<AccountGroup[]>({
    queryKey: ["accountGroups"],
    queryFn: api.ideAccounts.listGroups,
  });
  const { data: currentSnapshots = [] } = useQuery<CurrentAccountSnapshot[]>({
    queryKey: ["providerCurrentSnapshots"],
    queryFn: api.providerCurrent.listSnapshots,
    staleTime: 1000 * 15,
  });
  const currentIdeAccountIds = useMemo(
    () =>
      Object.fromEntries(
        currentSnapshots.map((item) => [item.platform, item.account_id ?? null])
      ) as Record<string, string | null>,
    [currentSnapshots]
  );

  const isLoading = keysLoading || ideLoading;

  // ---------- Mutations ----------
  const deleteKeyMut = useMutation({ mutationFn: api.keys.delete, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const checkKeyMut = useMutation({ mutationFn: api.keys.check, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const refreshBalMut = useMutation({ mutationFn: (id: string) => api.balance.refreshOne(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }) });
  const deleteIdeMut = useMutation({
    mutationFn: api.ideAccounts.delete,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      qc.invalidateQueries({ queryKey: ["accountGroups"] });
    },
  });
  const refreshIdeMut = useMutation({ mutationFn: api.ideAccounts.refresh, onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }) });
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
      for (const k of list) await api.keys.check(k.id);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] })
  });
  const statusActionMut = useMutation({
    mutationFn: (payload: { id: string; action: string; retryFailedTimes?: number | null }) =>
      api.ideAccounts.runStatusAction(payload.id, payload.action, payload.retryFailedTimes ?? null),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });

  // ---------- 侧边栏聚合 (Channels) 计算 ----------
  const channels = useMemo(() => {
    const apiMap = new Map<string, number>();
    const ideMap = new Map<string, number>();

    keys.forEach(k => apiMap.set(k.platform, (apiMap.get(k.platform) || 0) + 1));
    ideAccs.forEach(a => ideMap.set(a.origin_platform, (ideMap.get(a.origin_platform) || 0) + 1));

    const chs: ChannelMeta[] = [
      { id: "all", type: "all", label: "全部资产大盘", count: keys.length + ideAccs.length }
    ];

    // Standard APIs
    const apiChs: ChannelMeta[] = Array.from(apiMap.entries()).map(([plat, count]) => ({
      id: `api_${plat}`, type: "api" as ChannelType, count,
      label: PLATFORM_LABELS[plat as keyof typeof PLATFORM_LABELS] || plat
    })).sort((a, b) => b.count - a.count);

    // IDE Fingerprints
    const ideChs: ChannelMeta[] = Array.from(ideMap.entries()).map(([plat, count]) => ({
      id: `ide_${plat}`, type: "ide" as ChannelType, count,
      label: plat
    })).sort((a, b) => b.count - a.count);

    return { all: chs[0], apiChs, ideChs };
  }, [keys, ideAccs]);

  // ---------- 高密统一列表视图过筛 ----------
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

    // 根据左侧 Channel 过滤
    if (activeChannelId === "all") {
      rawItems = [
        ...keys.map((k): UnifiedAccountItem => ({ type: "api", data: k, balance: balanceMap[k.id] })),
        ...ideAccs.map((a): UnifiedAccountItem => ({ type: "ide", data: a }))
      ];
    } else if (activeChannelId.startsWith("api_")) {
      const plat = activeChannelId.replace("api_", "");
      rawItems = keys.filter(k => k.platform === plat).map(k => ({ type: "api", data: k, balance: balanceMap[k.id] }));
    } else if (activeChannelId.startsWith("ide_")) {
      const plat = activeChannelId.replace("ide_", "");
      rawItems = ideAccs.filter(a => a.origin_platform === plat).map(a => ({ type: "ide", data: a }));
    }

    // 根据搜索字符串二次过滤
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      rawItems = rawItems.filter(item => {
        if (item.type === "api") {
          return item.data.name.toLowerCase().includes(q) || item.data.platform.toLowerCase().includes(q);
        } else {
          return item.data.email.toLowerCase().includes(q) || item.data.origin_platform.toLowerCase().includes(q);
        }
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

    // 默认按最后使用或状态排序
    return rawItems.sort((a, b) => {
      const getT = (i: UnifiedAccountItem) => i.type === "api" ? new Date(i.data.created_at).getTime() : new Date((i.data as IdeAccount).last_used).getTime();
      return getT(b) - getT(a); 
    });
  }, [keys, ideAccs, balanceMap, activeChannelId, searchQuery, showAttentionOnly, attentionReasonFilter, accountGroupFilter, accountGroupByAccountId]);

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

  const itemRowHeight = accountViewMode === "compact" ? 42 : 52;
  const rowVirtualizer = useVirtualizer({
    count: accountViewMode === "grid" ? 0 : groupedRenderRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const row = groupedRenderRows[index];
      if (!row) return itemRowHeight;
      return row.type === "group" ? 36 : itemRowHeight;
    },
    overscan: accountViewMode === "compact" ? 14 : 10,
  });

  const togglePrivacy = () => {
    const next = !privacy;
    setPrivacy(next);
    setPrivacyMode(next);
  };

  const openConfirmDialog = (config: ConfirmDialogState) => {
    setConfirmDialog(config);
    setConfirmDialogBusy(false);
  };

  const activeChannelName = channels.all.id === activeChannelId ? "系统全局资产" : 
                            [...channels.apiChs, ...channels.ideChs].find(c => c.id === activeChannelId)?.label || "未知渠道";
  const isIdeProblemViewAvailable = activeChannelId === "all" || activeChannelId.startsWith("ide_");

  const totalFilteredCount = displayItems.length;
  const filteredIdeItems = displayItems.filter((item): item is { type: "ide"; data: IdeAccount } => item.type === "ide");
  const filteredIdeIds = filteredIdeItems.map((item) => item.data.id);
  const filteredDailyCheckinIdeItems = filteredIdeItems.filter((item) => supportsDailyCheckin(item.data));
  const filteredDailyCheckinIds = filteredDailyCheckinIdeItems.map((item) => item.data.id);
  const filteredAttentionIdeIds = filteredIdeItems.filter((item) => isIdeNeedsAttention(item.data)).map((item) => item.data.id);
  const filteredAttentionReasonIdeIds = attentionReasonFilter
    ? filteredIdeItems
        .filter((item) => isIdeMatchingAttentionReason(item.data, attentionReasonFilter))
        .map((item) => item.data.id)
    : [];
  const selectedVisibleIdeIds = selectedIdeIds.filter((id) => filteredIdeIds.includes(id));
  const canExportIdeAccounts = filteredIdeItems.length > 0 && (activeChannelId === "all" || activeChannelId.startsWith("ide_"));
  const canBatchEditIdeTags = filteredIdeItems.length > 0;
  const canBatchDailyCheckin = filteredDailyCheckinIds.length > 0;
  const batchRefreshablePlatforms = ["gemini", "codex", "cursor", "windsurf", "kiro", "qoder", "trae", "codebuddy", "codebuddy_cn", "workbuddy", "zed"];
  const activeIdePlatform = activeChannelId.startsWith("ide_") ? activeChannelId.replace("ide_", "") : null;
  const canBatchRefreshActiveIde = !!activeIdePlatform && batchRefreshablePlatforms.includes(activeIdePlatform);
  const selectedIdeCount = selectedVisibleIdeIds.length;
  const selectedVisibleIdeItems = filteredIdeItems.filter((item) => selectedVisibleIdeIds.includes(item.data.id));
  const canBatchGroupIde = filteredIdeItems.length > 0;
  const selectedIdePlatforms = [...new Set(selectedVisibleIdeItems.map((item) => item.data.origin_platform))];
  const canBatchSetCurrent = selectedIdeCount > 0 && selectedIdePlatforms.length === 1;
  const filteredIdePlatforms = [...new Set(filteredIdeItems.map((item) => item.data.origin_platform))];
  const canBatchSetCurrentForGroupView =
    accountGroupFilter !== "all" && filteredIdeIds.length > 0 && filteredIdePlatforms.length === 1;
  const selectedCurrentCount = selectedVisibleIdeItems.filter((item) => isCurrentIdeAccount(item.data, currentIdeAccountIds)).length;
  const attentionReasonLabel = attentionReasonFilter
    ? getAttentionReasonLabel(attentionReasonFilter)
    : null;
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

  useEffect(() => {
    if (!isIdeProblemViewAvailable && (showAttentionOnly || attentionReasonFilter)) {
      setShowAttentionOnly(false);
      setAttentionReasonFilter(null);
    }
    if (activeChannelId.startsWith("api_") && accountGroupFilter !== "all") {
      setAccountGroupFilter("all");
    }
  }, [isIdeProblemViewAvailable, showAttentionOnly, attentionReasonFilter, activeChannelId, accountGroupFilter]);

  useEffect(() => {
    persistAccountViewMode(accountViewMode);
  }, [accountViewMode]);

  useEffect(() => {
    persistTagGrouping(groupByTag);
  }, [groupByTag]);

  useEffect(() => {
    rowVirtualizer.measure();
  }, [groupedRenderRows, itemRowHeight, rowVirtualizer]);

  useEffect(() => {
    const scrollHost = parentRef.current;
    if (!scrollHost) return;
    scrollHost.scrollTop = 0;
    rowVirtualizer.scrollToOffset(0);
  }, [accountViewMode, groupByTag, activeChannelId, rowVirtualizer]);

  const ideOverview = useMemo(() => {
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
      const ts = new Date(item.data.last_used).getTime();
      return Number.isFinite(ts) && now - ts <= sevenDaysMs;
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
  }, [filteredIdeItems, currentIdeAccountIds]);

  const toggleIdeSelected = (id: string) => {
    setSelectedIdeIds((prev) => prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]);
  };

  const groupFilterBaseIdeItems = useMemo(() => {
    let items = ideAccs;
    if (activeChannelId.startsWith("ide_")) {
      const plat = activeChannelId.replace("ide_", "");
      items = items.filter((item) => item.origin_platform === plat);
    } else if (activeChannelId.startsWith("api_")) {
      return [] as IdeAccount[];
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      items = items.filter((item) =>
        item.email.toLowerCase().includes(q) ||
        item.origin_platform.toLowerCase().includes(q) ||
        (item.label || "").toLowerCase().includes(q)
      );
    }
    return items;
  }, [ideAccs, activeChannelId, searchQuery]);

  const groupFilterOptions = useMemo(() => {
    const usedGroupIds = new Set(
      groupFilterBaseIdeItems
        .map((item) => accountGroupByAccountId.get(item.id)?.id)
        .filter(Boolean)
    );
    return accountGroups.filter((group) => usedGroupIds.has(group.id));
  }, [groupFilterBaseIdeItems, accountGroups, accountGroupByAccountId]);


  const toggleAllVisibleIde = () => {
    if (filteredIdeIds.length > 0 && selectedVisibleIdeIds.length === filteredIdeIds.length) {
      setSelectedIdeIds((prev) => prev.filter((id) => !filteredIdeIds.includes(id)));
    } else {
      setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredIdeIds])]);
    }
  };

  const getItemDisplayName = (item: UnifiedAccountItem) => {
    if (privacy) {
      return item.type === "api" ? maskToken(item.data.key_preview) : maskEmail(item.data.email);
    }
    return item.type === "api" ? item.data.name : item.data.label?.trim() || item.data.email;
  };

  const getItemBalanceSummary = (item: UnifiedAccountItem) => {
    if (privacy) return "***";
    if (item.type === "api") {
      if (!item.balance) return "—";
      return item.balance.balance_usd != null
        ? `$${item.balance.balance_usd.toFixed(2)}`
        : `¥${item.balance.balance_cny?.toFixed(2) || "0.00"}`;
    }
    if (item.data.origin_platform === "gemini") return formatGeminiQuotaSummary(item.data.quota_json);
    if (item.data.origin_platform === "codex") return formatCodexQuotaSummary(item.data.quota_json);
    if (item.data.origin_platform === "cursor") return formatCursorSummary(item.data);
    if (item.data.origin_platform === "windsurf") return formatWindsurfSummary(item.data);
    if (item.data.origin_platform === "kiro") return formatKiroSummary(item.data);
    if (item.data.origin_platform === "qoder") return formatQoderSummary(item.data);
    if (item.data.origin_platform === "trae") return formatTraeSummary(item.data);
    if (item.data.origin_platform === "codebuddy" || item.data.origin_platform === "codebuddy_cn" || item.data.origin_platform === "workbuddy") {
      return formatCodebuddySummary(item.data);
    }
    return "—";
  };

  const getItemTimeSummary = (item: UnifiedAccountItem) => {
    if (item.type === "api") {
      return item.data.created_at ? new Date(item.data.created_at).toLocaleString() : "未知";
    }
    return item.data.last_used ? new Date(item.data.last_used).toLocaleString() : "从未调用";
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

  return (
    <div className="unified-accounts-page">
      
      {/* ─── 左侧轨道分栏 ─────────────────────────────── */}
      <div className="unified-sidebar">
        <div className="sidebar-brand">
          <Database size={20} color="var(--accent-primary, #2563eb)" />
          <span>资产仓库</span>
        </div>

        <div className="sidebar-section">
          <div 
            className={`channel-nav-item ${activeChannelId === "all" ? "active" : ""}`}
            onClick={() => setActiveChannelId("all")}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}><Box size={14}/> 全部账号</div>
            <span className="channel-count">{channels.all.count}</span>
          </div>
        </div>

        {channels.apiChs.length > 0 && (
          <div className="sidebar-section">
            <div className="sidebar-section-title">官方 API 渠道</div>
            {channels.apiChs.map(c => (
              <div 
                key={c.id} 
                className={`channel-nav-item ${activeChannelId === c.id ? "active" : ""}`}
                onClick={() => setActiveChannelId(c.id)}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}><Server size={14}/> {c.label}</div>
                <span className="channel-count">{c.count}</span>
              </div>
            ))}
          </div>
        )}

        {channels.ideChs.length > 0 && (
          <div className="sidebar-section">
            <div className="sidebar-section-title">IDE 沙盒池</div>
            {channels.ideChs.map(c => (
              <div 
                key={c.id} 
                className={`channel-nav-item ${activeChannelId === c.id ? "active" : ""}`}
                onClick={() => setActiveChannelId(c.id)}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}><ShieldCheck size={14}/> {c.label}</div>
                <span className="channel-count">{c.count}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ─── 右侧巨型工作区 ───────────────────────────── */}
      <div className="unified-main">
        {/* Header区 */}
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
            <button className={`btn-icon-label ${privacy ? "active" : ""}`} onClick={togglePrivacy}>
              {privacy ? <EyeOff size={15}/> : <Eye size={15}/>} {privacy ? "隐私开启" : "明文显示"}
            </button>
            {canBatchRefreshActiveIde && (
              <button
                className="btn-outline"
                onClick={() =>
                  refreshAllIdeByPlatformMut.mutate(activeIdePlatform!, {
                    onSuccess: (count) => setActionMessage({
                      text: `${activeIdePlatform} 批量刷新完成：成功 ${count} 个账号`,
                      tone: "success",
                    }),
                    onError: (e) => setActionMessage({
                      text: `${activeIdePlatform} 批量刷新失败: ` + e,
                      tone: "error",
                    }),
                  })
                }
                disabled={refreshAllIdeByPlatformMut.isPending}
              >
                <RefreshCw size={15} className={refreshAllIdeByPlatformMut.isPending ? "spin" : ""} />
                {refreshAllIdeByPlatformMut.isPending
                  ? "批量刷新中"
                  : `批量刷新 ${activeIdePlatform}`}
              </button>
            )}
            {canBatchDailyCheckin && (
              <button
                className="btn-outline"
                disabled={statusActionMut.isPending}
                onClick={async () => {
                  const targets = (selectedIdeCount > 0
                    ? selectedVisibleIdeItems.map((item) => item.data)
                    : filteredDailyCheckinIdeItems.map((item) => item.data))
                    .filter((item) => supportsDailyCheckin(item));
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
                }}
              >
                <CalendarCheck size={15} className={statusActionMut.isPending ? "spin" : ""} />
                {statusActionMut.isPending
                  ? "签到处理中"
                  : selectedIdeCount > 0
                    ? `签到已选 (${selectedIdeCount})`
                    : `一键签到 (${filteredDailyCheckinIds.length})`}
              </button>
            )}
            {canExportIdeAccounts && (
              <button
                className="btn-outline"
                onClick={async () => {
                  try {
                    const ids = selectedIdeCount > 0 ? selectedVisibleIdeIds : filteredIdeItems.map((item) => item.data.id);
                    const json = await api.ideAccounts.export(ids);
                    const blob = new Blob([json], { type: "application/json" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    const exportTag = activeChannelId === "all"
                      ? "ide-accounts"
                      : activeChannelId.replace("ide_", "") + "-accounts";
                    a.download = `${exportTag}-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                    setActionMessage({ text: `已导出 ${ids.length} 个 IDE 账号`, tone: "success" });
                  } catch (e) {
                    setActionMessage({ text: "导出 IDE 账号失败: " + e, tone: "error" });
                  }
                }}
              >
                <Share size={15} />
                {selectedIdeCount > 0 ? `导出已选 (${selectedIdeCount})` : "导出 IDE 账号"}
              </button>
            )}
            {canBatchEditIdeTags && (
              <button
                className="btn-outline"
                onClick={() => {
                  const targets = selectedIdeCount > 0
                    ? filteredIdeItems.filter((item) => selectedVisibleIdeIds.includes(item.data.id))
                    : filteredIdeItems;
                  const tagPool = [...new Set(targets.flatMap((item) => item.data.tags || []))];
                  setBatchIdeTagsDialog({
                    ids: targets.map((item) => item.data.id),
                    tagsText: tagPool.join(", "),
                    count: targets.length,
                    channelLabel: activeChannelName,
                  });
                }}
              >
                <Edit2 size={15} />
                {selectedIdeCount > 0 ? `批量标签 (${selectedIdeCount})` : "批量标签"}
              </button>
            )}
            {canBatchGroupIde && (
              <button
                className="btn-outline"
                onClick={() => {
                  const targets = selectedIdeCount > 0
                    ? selectedVisibleIdeIds
                    : filteredIdeItems.map((item) => item.data.id);
                  setAccountGroupDialog({
                    mode: "assign",
                    ids: targets,
                    count: targets.length,
                    channelLabel: activeChannelName,
                  });
                }}
              >
                <Folder size={15} />
                {selectedIdeCount > 0 ? `批量分组 (${selectedIdeCount})` : "批量分组"}
              </button>
            )}
            <button
              className="btn-outline"
              onClick={() =>
                setAccountGroupDialog({
                  mode: "manage",
                  ids: [],
                  count: 0,
                  channelLabel: activeChannelName,
                })
              }
            >
              <Folder size={15} />
              分组管理
            </button>
            <button className="btn-outline" onClick={() => {
                const keys = displayItems.map(i => i.data).filter(d => 'platform' in d) as ApiKey[];
                if(keys.length > 0) checkAllKeysMut.mutate(keys);
              }}
              disabled={checkAllKeysMut.isPending}
            >
              <RefreshCw size={15} className={checkAllKeysMut.isPending ? "spin" : ""} /> 
              {checkAllKeysMut.isPending ? "全量探测中" : "一键探测筛选键"}
            </button>
            <button className="btn-primary" onClick={() => setShowAddWizard(true)}>
              <Plus size={15} /> 添加资产
            </button>
          </div>
        </div>

        {/* 顶部搜挂区 */}
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
                  onClick={() => setAccountGroupFilter("all")}
                >
                  全部分组
                </button>
                <button
                  className={`btn-outline ${accountGroupFilter === "__ungrouped__" ? "active" : ""}`}
                  onClick={() => setAccountGroupFilter("__ungrouped__")}
                >
                  未分组
                </button>
                {groupFilterOptions.map((group) => (
                  <button
                    key={group.id}
                    className={`btn-outline ${accountGroupFilter === group.id ? "active" : ""}`}
                    onClick={() => setAccountGroupFilter(group.id)}
                  >
                    {group.name}
                  </button>
                ))}
                {accountGroupFilter !== "all" && (
                  <>
                    <button
                      className="btn-outline"
                      disabled={filteredIdeIds.length === 0}
                      onClick={() => setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredIdeIds])])}
                    >
                      一键只选当前分组
                    </button>
                    <button
                      className="btn-outline"
                      disabled={batchRefreshIdeMut.isPending || filteredIdeIds.length === 0}
                      onClick={() =>
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
                        })
                      }
                    >
                      <RefreshCw size={15} className={batchRefreshIdeMut.isPending ? "spin" : ""} />
                      {batchRefreshIdeMut.isPending ? "处理中..." : "刷新当前分组"}
                    </button>
                    <button
                      className="btn-outline"
                      disabled={filteredIdeIds.length === 0}
                      onClick={() => {
                        const tagPool = [...new Set(filteredIdeItems.flatMap((item) => item.data.tags || []))];
                        setBatchIdeTagsDialog({
                          ids: filteredIdeIds,
                          tagsText: tagPool.join(", "),
                          count: filteredIdeIds.length,
                          channelLabel: `${activeChannelName} · ${currentGroupActionLabel}`,
                        });
                      }}
                    >
                      <Edit2 size={15} />
                      打标当前分组
                    </button>
                    <button
                      className="btn-outline"
                      disabled={!canBatchSetCurrentForGroupView}
                      onClick={() =>
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
                        })
                      }
                    >
                      <MonitorPlay size={15} />
                      设为当前（分组）
                    </button>
                    <button
                      className="btn-outline"
                      disabled={filteredIdeIds.length === 0}
                      onClick={() =>
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
                        })
                      }
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
                  onClick={() => {
                    setShowAttentionOnly((prev) => !prev);
                    setAttentionReasonFilter(null);
                  }}
                >
                  {showAttentionOnly ? "显示全部 IDE" : "只看需关注"}
                </button>
                <button
                  className="btn-outline"
                  disabled={filteredAttentionIdeIds.length === 0}
                  onClick={() => setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredAttentionIdeIds])])}
                >
                  一键只选需关注
                </button>
                {attentionReasonFilter && (
                  <button
                    className="btn-outline"
                    disabled={filteredAttentionReasonIdeIds.length === 0}
                    onClick={() => setSelectedIdeIds((prev) => [...new Set([...prev, ...filteredAttentionReasonIdeIds])])}
                  >
                    一键只选{attentionReasonLabel}
                  </button>
                )}
                {(showAttentionOnly || attentionReasonFilter) && (
                  <button
                    className="btn-outline"
                    onClick={() => {
                      setShowAttentionOnly(false);
                      setAttentionReasonFilter(null);
                    }}
                  >
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
                {ideOverview.expiredCount > 0 && (
                  <button
                    className={`btn-outline ${attentionReasonFilter === "expired" ? "active" : ""}`}
                    onClick={() =>
                      setAttentionReasonFilter((prev) => (prev === "expired" ? null : "expired"))
                    }
                  >
                    只看过期
                  </button>
                )}
                {ideOverview.forbiddenCount > 0 && (
                  <button
                    className={`btn-outline ${attentionReasonFilter === "forbidden" ? "active" : ""}`}
                    onClick={() =>
                      setAttentionReasonFilter((prev) => (prev === "forbidden" ? null : "forbidden"))
                    }
                  >
                    只看封禁
                  </button>
                )}
                {ideOverview.rateLimitedCount > 0 && (
                  <button
                    className={`btn-outline ${attentionReasonFilter === "rate_limited" ? "active" : ""}`}
                    onClick={() =>
                      setAttentionReasonFilter((prev) => (prev === "rate_limited" ? null : "rate_limited"))
                    }
                  >
                    只看限流
                  </button>
                )}
                {ideOverview.proxyDisabledCount > 0 && (
                  <button
                    className={`btn-outline ${attentionReasonFilter === "proxy_disabled" ? "active" : ""}`}
                    onClick={() =>
                      setAttentionReasonFilter((prev) => (prev === "proxy_disabled" ? null : "proxy_disabled"))
                    }
                  >
                    只看代理禁用
                  </button>
                )}
                {ideOverview.manuallyDisabledCount > 0 && (
                  <button
                    className={`btn-outline ${attentionReasonFilter === "manually_disabled" ? "active" : ""}`}
                    onClick={() =>
                      setAttentionReasonFilter((prev) => (prev === "manually_disabled" ? null : "manually_disabled"))
                    }
                  >
                    只看人工禁用
                  </button>
                )}
              </div>
              {attentionReasonFilter && (
                <div className="accounts-overview-actions emphasis">
                  {canRefreshAttentionReason && (
                    <button
                      className="btn-outline"
                      disabled={batchRefreshIdeMut.isPending || filteredAttentionReasonIdeIds.length === 0}
                      onClick={() =>
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
                        })
                      }
                    >
                      <RefreshCw size={15} className={batchRefreshIdeMut.isPending ? "spin" : ""} />
                      {batchRefreshIdeMut.isPending ? "处理中..." : `刷新当前${attentionReasonLabel}`}
                    </button>
                  )}
                  {!canRefreshAttentionReason && (
                    <button
                      className="btn-outline"
                      disabled={filteredAttentionReasonIdeIds.length === 0}
                      onClick={() => {
                        const targets = filteredIdeItems.filter((item) =>
                          isIdeMatchingAttentionReason(item.data, attentionReasonFilter)
                        );
                        const tagPool = [...new Set(targets.flatMap((item) => item.data.tags || []))];
                        const suggestion = getAttentionReasonSuggestedTag(attentionReasonFilter);
                        const nextTags = [...new Set([...tagPool, suggestion])].join(", ");
                        setBatchIdeTagsDialog({
                          ids: targets.map((item) => item.data.id),
                          tagsText: nextTags,
                          count: targets.length,
                          channelLabel: `${activeChannelName} · ${attentionReasonLabel}`,
                        });
                      }}
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
          {filteredIdeIds.length > 0 && (
            <div className="accounts-selection-bar">
                <button className="btn-outline" onClick={toggleAllVisibleIde}>
                {selectedVisibleIdeIds.length === filteredIdeIds.length && filteredIdeIds.length > 0
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
                    onClick={() =>
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
                          setActionMessage({ text: `已依次切换 ${selectedIdeCount} 个 ${selectedIdePlatforms[0]} 账号，最后一个已成为当前账号`, tone: "success" });
                          qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
                        },
                      })
                    }
                    >
                      <MonitorPlay size={15} />
                      设为当前
                    </button>
                  {!canBatchSetCurrent && (
                    <span className="accounts-selection-text">批量设为当前仅支持同一平台的已选 IDE</span>
                  )}
                  <button
                    className="btn-outline"
                    onClick={() =>
                      batchRefreshIdeMut.mutate(selectedVisibleIdeIds, {
                        onSuccess: (count) => setActionMessage({ text: `已批量刷新 ${count} 个已选 IDE 账号`, tone: "success" }),
                        onError: (e) => setActionMessage({ text: "批量刷新已选 IDE 账号失败: " + e, tone: "error" }),
                      })
                    }
                    disabled={batchRefreshIdeMut.isPending}
                  >
                    <RefreshCw size={15} className={batchRefreshIdeMut.isPending ? "spin" : ""} />
                    {batchRefreshIdeMut.isPending ? "处理中..." : "刷新已选"}
                  </button>
                  <button
                    className="btn-outline"
                    onClick={() =>
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
                      })
                    }
                  >
                    <Trash2 size={15} />
                    删除已选
                  </button>
                  <button className="btn-outline" onClick={() => setSelectedIdeIds((prev) => prev.filter((id) => !selectedVisibleIdeIds.includes(id)))}>
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
              onClick={() => setAccountViewMode("list")}
            >
              列表
            </button>
            <button
              className={`btn-outline ${accountViewMode === "grid" ? "active" : ""}`}
              onClick={() => setAccountViewMode("grid")}
            >
              网格
            </button>
            <button
              className={`btn-outline ${accountViewMode === "compact" ? "active" : ""}`}
              onClick={() => setAccountViewMode("compact")}
            >
              紧凑
            </button>
            <button
              className={`btn-outline ${groupByTag ? "active" : ""}`}
              disabled={displayItems.length === 0}
              onClick={() => setGroupByTag((prev) => !prev)}
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
              onChange={e => setSearchQuery(e.target.value)}
            />
            {searchQuery && <X size={14} className="search-clear" onClick={() => setSearchQuery("")} style={{cursor: 'pointer'}} />}
          </div>
        </div>

        {/* 万级虚拟列表 */}
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
          ) : displayItems.length === 0 ? (
            <div className="empty-state">
              <Box size={32} opacity={0.5} />
              <span>{emptyStateMessage}</span>
              {currentProblemViewLabel && (
                <button
                  className="btn-outline"
                  onClick={() => {
                    setShowAttentionOnly(false);
                    setAttentionReasonFilter(null);
                  }}
                >
                  清除问题筛选
                </button>
              )}
              {accountGroupFilter !== "all" && (
                <button className="btn-outline" onClick={() => setAccountGroupFilter("all")}>
                  清除分组筛选
                </button>
              )}
              {searchQuery.trim() && (
                <button className="btn-outline" onClick={() => setSearchQuery("")}>
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
                    {section.items.map((item, index) => {
                      const isCurrent = item.type === "ide" ? isCurrentIdeAccount(item.data, currentIdeAccountIds) : false;
                      const groupName = item.type === "ide" ? accountGroupByAccountId.get(item.data.id)?.name : null;
                      const isIdeRefreshable = item.type === "ide" && isIdeRefreshSupported(item.data);

                      return (
                        <article key={`${item.type}:${item.data.id}:${index}`} className="account-grid-card">
                          <div className="account-grid-card-header">
                            <div className="row-icon">
                              {item.type === "api" ? <Server size={14} /> : <ShieldCheck size={14} />}
                            </div>
                            <div className="account-grid-title-wrap">
                              <div className="account-grid-title" title={item.type === "api" ? item.data.name : item.data.email}>
                                {getItemDisplayName(item)}
                              </div>
                              <div className="account-grid-subtitle">
                                {item.type === "api"
                                  ? (PLATFORM_LABELS[item.data.platform as keyof typeof PLATFORM_LABELS] || item.data.platform)
                                  : formatIdePlatformLabel(item.data)}
                              </div>
                            </div>
                          </div>
                          <div className="account-grid-badges">
                            <UnifiedAccountStatusCell item={item} />
                            {isCurrent && <span className="current-account-badge">当前</span>}
                            {groupName && <span className="current-account-badge account-group-badge">{groupName}</span>}
                          </div>
                          <div className="account-grid-meta">
                            <span>额度: {getItemBalanceSummary(item)}</span>
                            <span>时间: {getItemTimeSummary(item)}</span>
                          </div>
                          {item.type === "ide" && item.data.tags && item.data.tags.length > 0 && (
                            <div className="account-grid-tags">
                              {item.data.tags.slice(0, 3).map((tag) => (
                                <span key={tag} className="accounts-selection-chip">#{tag}</span>
                              ))}
                            </div>
                          )}
                          <div className="account-grid-actions">
                            {item.type === "ide" && (
                              <input
                                type="checkbox"
                                checked={selectedIdeIds.includes(item.data.id)}
                                onChange={() => toggleIdeSelected(item.data.id)}
                              />
                            )}
                            <button
                              className="btn-row-action"
                              title="快速生成分享 Token"
                              onClick={() =>
                                openConfirmDialog({
                                  title: "签发直连 Token",
                                  description: `为 ${item.type === "api" ? item.data.name : item.data.email} 单独签发一个透传 Token。`,
                                  confirmLabel: "立即签发",
                                  action: async () => {
                                    try {
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
                                    } catch (e) {
                                      setActionMessage({ text: "生成失败: " + e, tone: "error" });
                                      throw e;
                                    }
                                  },
                                })
                              }
                            >
                              <Share size={14} />
                            </button>
                            {item.type === "ide" && supportsDailyCheckin(item.data) && (
                              <button
                                className="btn-row-action"
                                title="执行每日签到（失败自动重试 1 次）"
                                onClick={() => runDailyCheckinForAccount(item.data, 1)}
                                disabled={statusActionMut.isPending}
                              >
                                <CalendarCheck size={14} />
                              </button>
                            )}
                            {item.type === "api" ? (
                              <>
                                <button className="btn-row-action" onClick={() => refreshBalMut.mutate(item.data.id)} title="刷新余额">
                                  <RefreshCw size={14} />
                                </button>
                                <button className="btn-row-action" onClick={() => checkKeyMut.mutate(item.data.id)} title="探测连通性">
                                  <MonitorPlay size={14} />
                                </button>
                              </>
                            ) : (
                              <>
                                <button
                                  className="btn-row-action"
                                  disabled={isCurrent}
                                  title={isCurrent ? "当前账号" : getCurrentActionLabel(item.data)}
                                  onClick={() =>
                                    openConfirmDialog({
                                      title: getCurrentActionLabel(item.data),
                                      description: isCurrent
                                        ? `${item.data.email} 已经是当前账号。`
                                        : `确认将 ${item.data.email} 设为当前本地账号吗？`,
                                      confirmLabel: "立即切换",
                                      action: async () => {
                                        await api.ideAccounts.forceInject(item.data.id);
                                        qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
                                        setActionMessage({ text: `${getCurrentActionLabel(item.data)}成功`, tone: "success" });
                                      },
                                    })
                                  }
                                >
                                  <MonitorPlay size={14} />
                                </button>
                                {isIdeRefreshable && (
                                  <button
                                    className="btn-row-action"
                                    onClick={() =>
                                      refreshIdeMut.mutate(item.data.id, {
                                        onSuccess: () =>
                                          setActionMessage({
                                            text: getIdeRefreshSuccessMessage(item.data.origin_platform),
                                            tone: "success",
                                          }),
                                        onError: (e) =>
                                          setActionMessage({
                                            text: getIdeRefreshFailureMessage(item.data.origin_platform, e),
                                            tone: "error",
                                          }),
                                      })
                                    }
                                    title={getIdeRefreshActionLabel(item.data.origin_platform)}
                                  >
                                    <RefreshCw size={14} className={refreshIdeMut.isPending ? "spin" : ""} />
                                  </button>
                                )}
                              </>
                            )}
                          </div>
                        </article>
                      );
                    })}
                  </div>
                </section>
              ))}
            </div>
          ) : (
            <div className="virtual-list-inner" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
              {rowVirtualizer.getVirtualItems().map(virtualRow => {
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
                  <div
                    key={row.key}
                    className={`data-table-row ${accountViewMode === "compact" ? "compact" : ""}`}
                    data-index={virtualRow.index}
                    style={{
                      height: `${virtualRow.size}px`,
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    {/* ID & Name */}
                    <div className="col-id row-identity table-cell-ellipsis">
                      {item.type === "ide" && (
                        <input
                          type="checkbox"
                          checked={selectedIdeIds.includes(item.data.id)}
                          onChange={() => toggleIdeSelected(item.data.id)}
                          onClick={(e) => e.stopPropagation()}
                        />
                      )}
                      <div className="row-icon">
                        {item.type === "api" ? <Server size={14} /> : <ShieldCheck size={14} />}
                      </div>
                      <span title={item.type === "api" ? item.data.name : item.data.email}>
                      {privacy 
                          ? (item.type === "api" ? maskToken(item.data.key_preview) : maskEmail(item.data.email))
                          : (item.type === "api" ? item.data.name : (item.data.label?.trim() || item.data.email))
                        }
                      </span>
                      {item.type === "ide" && isCurrentIdeAccount(item.data, currentIdeAccountIds) && (
                        <span className="current-account-badge">当前</span>
                      )}
                      {item.type === "ide" && accountGroupByAccountId.get(item.data.id) && (
                        <span className="current-account-badge account-group-badge">
                          {accountGroupByAccountId.get(item.data.id)?.name}
                        </span>
                      )}
                    </div>

                    {/* Platform */}
                    <div
                      className="col-platform table-cell-ellipsis text-muted"
                      title={item.type === "ide" && item.data.origin_platform === "codex" ? formatCodexApiKeyTooltip(item.data) : undefined}
                    >
                      {item.type === "api"
                        ? (PLATFORM_LABELS[item.data.platform as keyof typeof PLATFORM_LABELS] || item.data.platform)
                        : formatIdePlatformLabel(item.data)}
                    </div>

                    {/* Status */}
                    <div className="col-status">
                      <UnifiedAccountStatusCell item={item} />
                    </div>

                    {/* Balance */}
                    <div className="col-balance table-cell-ellipsis">
                      <UnifiedAccountBalanceCell item={item} privacy={privacy} />
                    </div>

                    {/* Time */}
                    <div className="col-time table-cell-ellipsis">
                      {item.type === "api" 
                        ? (item.data.created_at ? new Date(item.data.created_at).toLocaleString() : "未知")
                        : ((item.data as IdeAccount).last_used ? new Date((item.data as IdeAccount).last_used).toLocaleString() : "从未调用")
                      }
                    </div>

                    {/* Actions */}
                    <div className="col-actions">
                      <button className="btn-row-action" onClick={async () => {
                        openConfirmDialog({
                          title: "签发直连 Token",
                          description: `为 ${item.type==='api'?item.data.name:item.data.email} 单独签发一个透传 Token。`,
                          confirmLabel: "立即签发",
                          action: async () => {
                            try {
                              await api.userTokens.create({
                                username: `[极速生成] ${item.type==='api'?item.data.name:item.data.email}`,
                                description: JSON.stringify({ desc: "单点直连专用", scope: "single", single_account: item.data.id }),
                                expires_type: "never", expires_at: null, max_ips: 0, curfew_start: null, curfew_end: null
                              });
                              setActionMessage({ text: "已生成底座专属直连 Token，请切换至【分享额度】页面查看。", tone: "success" });
                            } catch (e) {
                              setActionMessage({ text: "生成失败: " + e, tone: "error" });
                              throw e;
                            }
                          },
                        });
                      }} title="快速生成分享 Token"><Share size={14}/></button>
                      {item.type === "ide" && supportsDailyCheckin(item.data) && (
                        <button
                          className="btn-row-action"
                          onClick={() => runDailyCheckinForAccount(item.data, 1)}
                          title="执行每日签到（失败自动重试 1 次）"
                          disabled={statusActionMut.isPending}
                        >
                          <CalendarCheck size={14} />
                        </button>
                      )}

                      {item.type === "api" ? (
                        <>
                          <button className="btn-row-action" onClick={() => refreshBalMut.mutate(item.data.id)} title="刷新余额"><RefreshCw size={14}/></button>
                          <button className="btn-row-action" onClick={() => checkKeyMut.mutate(item.data.id)} title="探测连通性"><MonitorPlay size={14}/></button>
                          <button className="btn-row-action danger" onClick={() => {
                            openConfirmDialog({
                              title: "删除密钥",
                              description: `确认删除密钥 ${item.data.name} 吗？此操作无法撤销。`,
                              confirmLabel: "删除",
                              tone: "danger",
                              action: async () => {
                                deleteKeyMut.mutate(item.data.id);
                              },
                            });
                          }} title="彻底销毁"><X size={14}/></button>
                        </>
                      ) : (
                        <>
                          {(() => {
                            const isCurrent = isCurrentIdeAccount(item.data, currentIdeAccountIds);
                            const actionLabel = getCurrentActionLabel(item.data);
                            return (
                              <button
                                className="btn-row-action"
                                disabled={isCurrent}
                                onClick={async () => {
                                  openConfirmDialog({
                                    title: actionLabel,
                                    description: isCurrent
                                      ? `${item.data.email} 已经是当前账号。`
                                      : `确认将 ${item.data.email} 设为当前本地账号吗？`,
                                    confirmLabel: "立即切换",
                                    action: async () => {
                                      await api.ideAccounts.forceInject(item.data.id)
                                        .then(() => {
                                          setActionMessage({ text: `${actionLabel}成功`, tone: "success" });
                                          qc.invalidateQueries({ queryKey: ["providerCurrentSnapshots"] });
                                        })
                                        .catch(e => {
                                          setActionMessage({ text: String(e), tone: "error" });
                                          throw e;
                                        });
                                    },
                                  });
                                }}
                                title={isCurrent ? "当前账号" : actionLabel}
                              >
                                <MonitorPlay size={14}/>
                              </button>
                            );
                          })()}
                          {isIdeRefreshSupported(item.data) && (
                            <button
                              className="btn-row-action"
                              onClick={() => {
                                refreshIdeMut.mutate(item.data.id, {
                                  onSuccess: () =>
                                    setActionMessage({
                                      text: getIdeRefreshSuccessMessage(item.data.origin_platform),
                                      tone: "success",
                                    }),
                                  onError: (e) =>
                                    setActionMessage({
                                      text: getIdeRefreshFailureMessage(item.data.origin_platform, e),
                                      tone: "error",
                                    }),
                                });
                              }}
                              title={getIdeRefreshActionLabel(item.data.origin_platform)}
                            >
                              <RefreshCw size={14} className={refreshIdeMut.isPending ? "spin" : ""} />
                            </button>
                          )}
                          {item.data.origin_platform === "gemini" && (
                            <button
                              className="btn-row-action"
                              onClick={async () => {
                                try {
                                  const projects = await api.ideAccounts.listGeminiProjects(item.data.id);
                                  if (projects.length === 0) {
                                    setActionMessage({ text: "当前账号没有可选的 Gemini Cloud 项目", tone: "info" });
                                    return;
                                  }
                                  setGeminiProjectDialog({
                                    account: item.data,
                                    projects,
                                    value: item.data.project_id || projects[0]?.project_id || "",
                                  });
                                } catch (e) {
                                  setActionMessage({ text: "设置 Gemini 项目失败: " + e, tone: "error" });
                                }
                              }}
                              title="设置 Gemini 项目"
                            >
                              <Folder size={14} />
                            </button>
                          )}
                          {item.data.origin_platform === "codex" && isCodexApiKeyAccount(item.data) && (
                            <button
                              className="btn-row-action"
                              onClick={async () => {
                                const meta = parseIdeMeta(item.data.meta_json);
                                setCodexApiKeyDialog({
                                  account: item.data,
                                  apiKey: typeof meta.openai_api_key === "string" ? meta.openai_api_key : "",
                                  baseUrl: typeof meta.api_base_url === "string" ? meta.api_base_url : "",
                                });
                              }}
                              title="编辑 Codex API Key 凭证"
                            >
                              <Key size={14} />
                            </button>
                          )}
                          <button
                            className="btn-row-action"
                            onClick={() => setIdeLabelDialog({
                              account: item.data,
                              label: item.data.label || "",
                            })}
                            title="编辑备注名"
                          >
                            <Edit2 size={14} />
                          </button>
                          <button className="btn-row-action danger" onClick={() => {
                            openConfirmDialog({
                              title: "删除指纹账号",
                              description: `确认删除 ${item.data.email} 吗？此操作无法撤销。`,
                              confirmLabel: "删除",
                              tone: "danger",
                              action: async () => {
                                deleteIdeMut.mutate(item.data.id);
                              },
                            });
                          }} title="拔除资产"><X size={14}/></button>
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {showAddWizard && (
        <AddAccountWizard
          onClose={() => setShowAddWizard(false)}
          onSuccess={() => {
            setShowAddWizard(false);
            qc.invalidateQueries({ queryKey: ["keys"] });
            qc.invalidateQueries({ queryKey: ["ideAccounts"] });
            qc.invalidateQueries({ queryKey: ["accountGroups"] });
          }}
        />
      )}

      <ConfirmDialogModal
        dialog={confirmDialog}
        busy={confirmDialogBusy}
        setBusy={setConfirmDialogBusy}
        onClose={() => setConfirmDialog(null)}
      />

      <GeminiProjectDialogModal
        dialog={geminiProjectDialog}
        setDialog={setGeminiProjectDialog}
        busy={geminiProjectBusy}
        onClose={() => setGeminiProjectDialog(null)}
        onClear={handleClearGeminiProject}
        onSave={handleSaveGeminiProject}
      />

      <CodexApiKeyDialogModal
        dialog={codexApiKeyDialog}
        setDialog={setCodexApiKeyDialog}
        busy={codexApiKeyBusy}
        onClose={() => setCodexApiKeyDialog(null)}
        onSave={handleSaveCodexApiKey}
      />

      <IdeLabelDialogModal
        dialog={ideLabelDialog}
        setDialog={setIdeLabelDialog}
        busy={ideLabelBusy}
        onClose={() => setIdeLabelDialog(null)}
        onSave={handleSaveIdeLabel}
      />

      <BatchIdeTagsDialogModal
        dialog={batchIdeTagsDialog}
        setDialog={setBatchIdeTagsDialog}
        busy={batchIdeTagsBusy}
        onClose={() => setBatchIdeTagsDialog(null)}
        onSave={handleSaveBatchIdeTags}
      />

      {accountGroupDialog && (
        <div className="accounts-modal-overlay" onClick={() => !accountGroupBusy && setAccountGroupDialog(null)}>
          <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="accounts-modal-title">
              {accountGroupDialog.mode === "manage" ? "账号分组管理" : "批量账号分组"}
            </h3>
            <p className="accounts-modal-desc">
              {accountGroupDialog.mode === "manage"
                ? "维护 IDE 账号分组，并为后续批量操作与调度筛选提供基础。"
                : `当前将处理 ${accountGroupDialog.channelLabel} 下的 ${accountGroupDialog.count} 个 IDE 账号。`}
            </p>

            <div className="accounts-form-group">
              <label className="accounts-form-label">新建分组</label>
              <div className="accounts-inline-row">
                <input
                  className="accounts-form-input"
                  placeholder="例如：主力 / 备用 / 待验证"
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  disabled={accountGroupBusy}
                />
                <button
                  className="btn-primary"
                  disabled={accountGroupBusy || !newGroupName.trim()}
                  onClick={async () => {
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
                  }}
                >
                  创建
                </button>
              </div>
            </div>

            <div className="accounts-group-list">
              {accountGroups.length === 0 ? (
                <div className="empty-text">当前还没有账号分组</div>
              ) : (
                accountGroups.map((group) => (
                  <div key={group.id} className="accounts-group-item">
                    <div className="accounts-group-main">
                      {renamingGroupId === group.id ? (
                        <input
                          className="accounts-form-input"
                          value={renamingGroupName}
                          onChange={(e) => setRenamingGroupName(e.target.value)}
                          disabled={accountGroupBusy}
                        />
                      ) : (
                        <div className="accounts-group-name">{group.name}</div>
                      )}
                      <div className="accounts-group-meta">{group.account_ids.length} 个账号</div>
                    </div>
                    <div className="accounts-group-actions">
                      {accountGroupDialog.mode === "assign" && (
                        <>
                          <button
                            className="btn-outline"
                            disabled={accountGroupBusy || accountGroupDialog.ids.length === 0}
                            onClick={async () => {
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
                            }}
                          >
                            加入分组
                          </button>
                          <button
                            className="btn-outline"
                            disabled={accountGroupBusy || accountGroupDialog.ids.length === 0}
                            onClick={async () => {
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
                            }}
                          >
                            移出分组
                          </button>
                        </>
                      )}
                      {accountGroupDialog.mode === "manage" && (
                        <>
                          {renamingGroupId === group.id ? (
                            <>
                              <button
                                className="btn-outline"
                                disabled={accountGroupBusy || !renamingGroupName.trim()}
                                onClick={async () => {
                                  try {
                                    setAccountGroupBusy(true);
                                    await api.ideAccounts.renameGroup(group.id, renamingGroupName.trim());
                                    qc.invalidateQueries({ queryKey: ["accountGroups"] });
                                    setActionMessage({ text: "账号分组已重命名", tone: "success" });
                                    setRenamingGroupId(null);
                                    setRenamingGroupName("");
                                  } catch (e) {
                                    setActionMessage({ text: "重命名账号分组失败: " + e, tone: "error" });
                                  } finally {
                                    setAccountGroupBusy(false);
                                  }
                                }}
                              >
                                保存
                              </button>
                              <button
                                className="btn-outline"
                                disabled={accountGroupBusy}
                                onClick={() => {
                                  setRenamingGroupId(null);
                                  setRenamingGroupName("");
                                }}
                              >
                                取消
                              </button>
                            </>
                          ) : (
                            <button
                              className="btn-outline"
                              onClick={() => {
                                setRenamingGroupId(group.id);
                                setRenamingGroupName(group.name);
                              }}
                            >
                              重命名
                            </button>
                          )}
                          <button
                            className="btn-danger-solid"
                            disabled={accountGroupBusy}
                            onClick={async () => {
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
                            }}
                          >
                            删除
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>

            <div className="accounts-modal-actions">
              <button className="btn-outline" onClick={() => setAccountGroupDialog(null)} disabled={accountGroupBusy}>
                关闭
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
