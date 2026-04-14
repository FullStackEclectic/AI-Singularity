import { useState, useMemo, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "../../lib/api";
import { PLATFORM_LABELS, STATUS_LABELS } from "../../types";
import type { ApiKey, IdeAccount, Balance } from "../../types";
import AddAccountWizard from "./AddAccountWizard";
import { isPrivacyMode, setPrivacyMode, maskEmail, maskToken } from "../../lib/privacyMode";
import { 
  Database, Server, ShieldCheck, Box, 
  Search, Eye, EyeOff, RefreshCw, Plus, X, MonitorPlay, Share, Folder, Key, Edit2
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

type ConfirmDialogState = {
  title: string;
  description: string;
  confirmLabel: string;
  tone?: "danger" | "primary";
  action: () => Promise<void> | void;
};

type GeminiProjectDialogState = {
  account: IdeAccount;
  projects: { project_id: string; project_name?: string | null }[];
  value: string;
};

type CodexApiKeyDialogState = {
  account: IdeAccount;
  apiKey: string;
  baseUrl: string;
};

type IdeLabelDialogState = {
  account: IdeAccount;
  label: string;
};

type UnifiedAccountItem = 
  | { type: "api"; data: ApiKey; balance?: Balance }
  | { type: "ide"; data: IdeAccount };

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
  const currentAwarePlatforms = useMemo(
    () =>
      [...new Set(ideAccs.map((item) => item.origin_platform))]
        .filter((platform) => [
          "codex",
          "gemini",
          "cursor",
          "windsurf",
          "kiro",
          "codebuddy",
          "codebuddy_cn",
          "workbuddy",
          "qoder",
          "trae",
          "zed",
        ].includes(platform)),
    [ideAccs]
  );
  const { data: currentIdeAccountIds = {} } = useQuery({
    queryKey: ["providerCurrentAccounts", currentAwarePlatforms],
    queryFn: async () => {
      const entries = await Promise.all(
        currentAwarePlatforms.map(async (platform) => [
          platform,
          await api.providerCurrent.getAccountId(platform).catch(() => null),
        ] as const)
      );
      return Object.fromEntries(entries) as Record<string, string | null>;
    },
    staleTime: 1000 * 15,
    enabled: currentAwarePlatforms.length > 0,
  });

  const isLoading = keysLoading || ideLoading;

  // ---------- Mutations ----------
  const deleteKeyMut = useMutation({ mutationFn: api.keys.delete, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const checkKeyMut = useMutation({ mutationFn: api.keys.check, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const refreshBalMut = useMutation({ mutationFn: (id: string) => api.balance.refreshOne(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }) });
  const deleteIdeMut = useMutation({ mutationFn: api.ideAccounts.delete, onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }) });
  const refreshIdeMut = useMutation({ mutationFn: api.ideAccounts.refresh, onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }) });
  const refreshAllIdeByPlatformMut = useMutation({
    mutationFn: api.ideAccounts.refreshAllByPlatform,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });
  const checkAllKeysMut = useMutation({
    mutationFn: async (list: ApiKey[]) => {
      for (const k of list) await api.keys.check(k.id);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] })
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

    // 默认按最后使用或状态排序
    return rawItems.sort((a, b) => {
      const getT = (i: UnifiedAccountItem) => i.type === "api" ? new Date(i.data.created_at).getTime() : new Date((i.data as IdeAccount).last_used).getTime();
      return getT(b) - getT(a); 
    });
  }, [keys, ideAccs, balanceMap, activeChannelId, searchQuery]);

  // ---------- 虚拟列表引擎初始化 ----------
  const rowVirtualizer = useVirtualizer({
    count: displayItems.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 52, // 单行高度预估 52px
    overscan: 10,
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

  const totalFilteredCount = displayItems.length;
  const filteredIdeItems = displayItems.filter((item): item is { type: "ide"; data: IdeAccount } => item.type === "ide");
  const canExportIdeAccounts = filteredIdeItems.length > 0 && (activeChannelId === "all" || activeChannelId.startsWith("ide_"));
  const batchRefreshablePlatforms = ["gemini", "codex", "cursor", "windsurf", "kiro", "qoder", "trae"];
  const activeIdePlatform = activeChannelId.startsWith("ide_") ? activeChannelId.replace("ide_", "") : null;
  const canBatchRefreshActiveIde = !!activeIdePlatform && batchRefreshablePlatforms.includes(activeIdePlatform);

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
            <p className="main-subtitle">已筛选 {totalFilteredCount} 条可用账单记录</p>
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
            {canExportIdeAccounts && (
              <button
                className="btn-outline"
                onClick={async () => {
                  try {
                    const ids = filteredIdeItems.map((item) => item.data.id);
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
                    setActionMessage({ text: `已导出 ${filteredIdeItems.length} 个 IDE 账号`, tone: "success" });
                  } catch (e) {
                    setActionMessage({ text: "导出 IDE 账号失败: " + e, tone: "error" });
                  }
                }}
              >
                <Share size={15} />
                导出 IDE 账号
              </button>
            )}
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
        <div className="table-container" ref={parentRef}>
          {/* Table Header (Sticky) */}
          <div className="data-table-header">
            <div className="col-id">标识符 (UID)</div>
            <div className="col-platform">渠道类型</div>
            <div className="col-status">状态</div>
            <div className="col-balance">剩余额度</div>
            <div className="col-time">最后使用/心跳</div>
            <div className="col-actions">高危操作</div>
          </div>

          {isLoading ? (
            <div className="empty-state">
              <RefreshCw size={24} className="spin" />
              <span>核心数据网络拉取中...</span>
            </div>
          ) : displayItems.length === 0 ? (
            <div className="empty-state">
              <Box size={32} opacity={0.5} />
              <span>当前汇聚池为空或被筛选掉</span>
            </div>
          ) : (
            <div className="virtual-list-inner" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
              {rowVirtualizer.getVirtualItems().map(virtualRow => {
                const item = displayItems[virtualRow.index];
                return (
                  <div
                    key={virtualRow.index}
                    className="data-table-row"
                    data-index={virtualRow.index}
                    style={{
                      height: `${virtualRow.size}px`,
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    {/* ID & Name */}
                    <div className="col-id row-identity table-cell-ellipsis">
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
                      <StatusCell item={item} />
                    </div>

                    {/* Balance */}
                    <div className="col-balance table-cell-ellipsis">
                      {item.type === "api" && item.balance ? (
                        <span className="text-success" style={{ fontWeight: 600 }}>
                           {privacy ? "***" : (item.balance.balance_usd != null ? `$${item.balance.balance_usd.toFixed(2)}` : `¥${item.balance.balance_cny?.toFixed(2) || '0.00'}`)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "gemini" && item.data.quota_json ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatGeminiQuotaTooltip(item.data.quota_json)}
                        >
                          {privacy ? "***" : formatGeminiQuotaSummary(item.data.quota_json)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "codex" && item.data.quota_json ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatCodexQuotaTooltip(item.data.quota_json)}
                        >
                          {privacy ? "***" : formatCodexQuotaSummary(item.data.quota_json)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "cursor" ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatCursorTooltip(item.data)}
                        >
                          {privacy ? "***" : formatCursorSummary(item.data)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "windsurf" ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatWindsurfTooltip(item.data)}
                        >
                          {privacy ? "***" : formatWindsurfSummary(item.data)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "kiro" ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatKiroTooltip(item.data)}
                        >
                          {privacy ? "***" : formatKiroSummary(item.data)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "qoder" ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatQoderTooltip(item.data)}
                        >
                          {privacy ? "***" : formatQoderSummary(item.data)}
                        </span>
                      ) : item.type === "ide" && item.data.origin_platform === "trae" ? (
                        <span
                          className="text-success"
                          style={{ fontWeight: 600 }}
                          title={privacy ? undefined : formatTraeTooltip(item.data)}
                        >
                          {privacy ? "***" : formatTraeSummary(item.data)}
                        </span>
                      ) : <span className="text-muted">—</span>}
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
                                          qc.invalidateQueries({ queryKey: ["providerCurrentAccounts"] });
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
                          {((["gemini", "cursor", "windsurf", "kiro", "qoder", "trae"].includes(item.data.origin_platform)) || (item.data.origin_platform === "codex" && !isCodexApiKeyAccount(item.data))) && (
                            <button
                              className="btn-row-action"
                              onClick={() => {
                                refreshIdeMut.mutate(item.data.id, {
                                  onSuccess: () => setActionMessage({
                                    text:
                                      item.data.origin_platform === "codex"
                                        ? "Codex 状态与配额已刷新"
                                        : item.data.origin_platform === "gemini"
                                          ? "Gemini 状态与配额已刷新"
                                          : item.data.origin_platform === "cursor"
                                            ? "Cursor 本地登录态已刷新"
                                            : item.data.origin_platform === "windsurf"
                                              ? "Windsurf 本地登录态已刷新"
                                              : item.data.origin_platform === "kiro"
                                                ? "Kiro 本地登录态已刷新"
                                                : item.data.origin_platform === "qoder"
                                                  ? "Qoder 本地登录态已刷新"
                                                  : "Trae 本地登录态已刷新",
                                    tone: "success",
                                  }),
                                  onError: (e) => setActionMessage({
                                    text:
                                      item.data.origin_platform === "codex"
                                        ? "Codex 刷新失败: " + e
                                        : item.data.origin_platform === "gemini"
                                          ? "Gemini 刷新失败: " + e
                                          : item.data.origin_platform === "cursor"
                                            ? "Cursor 刷新失败: " + e
                                            : item.data.origin_platform === "windsurf"
                                              ? "Windsurf 刷新失败: " + e
                                              : item.data.origin_platform === "kiro"
                                                ? "Kiro 刷新失败: " + e
                                                : item.data.origin_platform === "qoder"
                                                  ? "Qoder 刷新失败: " + e
                                                  : "Trae 刷新失败: " + e,
                                    tone: "error",
                                  }),
                                });
                              }}
                              title={
                                item.data.origin_platform === "codex"
                                  ? "刷新 Codex 配额与资料"
                                  : item.data.origin_platform === "gemini"
                                    ? "刷新 Gemini 状态与配额"
                                    : item.data.origin_platform === "cursor"
                                      ? "刷新 Cursor 本地登录态"
                                      : item.data.origin_platform === "windsurf"
                                        ? "刷新 Windsurf 本地登录态"
                                        : item.data.origin_platform === "kiro"
                                          ? "刷新 Kiro 本地登录态"
                                          : item.data.origin_platform === "qoder"
                                            ? "刷新 Qoder 本地登录态"
                                            : "刷新 Trae 本地登录态"
                              }
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
          }}
        />
      )}

      {confirmDialog && (
        <div className="accounts-modal-overlay" onClick={() => !confirmDialogBusy && setConfirmDialog(null)}>
          <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="accounts-modal-title">{confirmDialog.title}</h3>
            <p className="accounts-modal-desc">{confirmDialog.description}</p>
            <div className="accounts-modal-actions">
              <button
                className="btn-outline"
                onClick={() => setConfirmDialog(null)}
                disabled={confirmDialogBusy}
              >
                取消
              </button>
              <button
                className={confirmDialog.tone === "danger" ? "btn-danger-solid" : "btn-primary"}
                onClick={async () => {
                  try {
                    setConfirmDialogBusy(true);
                    await confirmDialog.action();
                    setConfirmDialog(null);
                  } finally {
                    setConfirmDialogBusy(false);
                  }
                }}
                disabled={confirmDialogBusy}
              >
                {confirmDialogBusy ? "处理中..." : confirmDialog.confirmLabel}
              </button>
            </div>
          </div>
        </div>
      )}

      {geminiProjectDialog && (
        <div className="accounts-modal-overlay" onClick={() => !geminiProjectBusy && setGeminiProjectDialog(null)}>
          <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="accounts-modal-title">设置 Gemini 项目</h3>
            <p className="accounts-modal-desc">{geminiProjectDialog.account.email}</p>
            <div className="accounts-form-group">
              <label className="accounts-form-label">可选项目</label>
              <select
                className="accounts-form-input"
                value={geminiProjectDialog.value}
                onChange={(e) => setGeminiProjectDialog((prev) => prev ? { ...prev, value: e.target.value } : prev)}
                disabled={geminiProjectBusy}
              >
                {geminiProjectDialog.projects.map((project) => (
                  <option key={project.project_id} value={project.project_id}>
                    {project.project_id}{project.project_name ? ` (${project.project_name})` : ""}
                  </option>
                ))}
              </select>
            </div>
            <div className="accounts-form-group">
              <label className="accounts-form-label">或手动输入 project_id</label>
              <input
                className="accounts-form-input"
                value={geminiProjectDialog.value}
                onChange={(e) => setGeminiProjectDialog((prev) => prev ? { ...prev, value: e.target.value } : prev)}
                disabled={geminiProjectBusy}
              />
            </div>
            <div className="accounts-modal-actions">
              <button className="btn-outline" onClick={() => setGeminiProjectDialog(null)} disabled={geminiProjectBusy}>取消</button>
              <button
                className="btn-primary"
                disabled={geminiProjectBusy || !geminiProjectDialog.value.trim()}
                onClick={async () => {
                  try {
                    setGeminiProjectBusy(true);
                    const selectedProjectId = geminiProjectDialog.value.trim();
                    await api.ideAccounts.setGeminiProject(geminiProjectDialog.account.id, selectedProjectId);
                    setActionMessage({ text: `已绑定 Gemini 项目：${selectedProjectId}`, tone: "success" });
                    qc.invalidateQueries({ queryKey: ["ideAccounts"] });
                    setGeminiProjectDialog(null);
                  } catch (e) {
                    setActionMessage({ text: "设置 Gemini 项目失败: " + e, tone: "error" });
                  } finally {
                    setGeminiProjectBusy(false);
                  }
                }}
              >
                {geminiProjectBusy ? "保存中..." : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}

      {codexApiKeyDialog && (
        <div className="accounts-modal-overlay" onClick={() => !codexApiKeyBusy && setCodexApiKeyDialog(null)}>
          <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="accounts-modal-title">编辑 Codex API Key</h3>
            <p className="accounts-modal-desc">{codexApiKeyDialog.account.email}</p>
            <div className="accounts-form-group">
              <label className="accounts-form-label">API Key</label>
              <input
                className="accounts-form-input"
                value={codexApiKeyDialog.apiKey}
                onChange={(e) => setCodexApiKeyDialog((prev) => prev ? { ...prev, apiKey: e.target.value } : prev)}
                disabled={codexApiKeyBusy}
              />
            </div>
            <div className="accounts-form-group">
              <label className="accounts-form-label">Base URL</label>
              <input
                className="accounts-form-input"
                placeholder="留空表示清空自定义 Base URL"
                value={codexApiKeyDialog.baseUrl}
                onChange={(e) => setCodexApiKeyDialog((prev) => prev ? { ...prev, baseUrl: e.target.value } : prev)}
                disabled={codexApiKeyBusy}
              />
            </div>
            <div className="accounts-modal-actions">
              <button className="btn-outline" onClick={() => setCodexApiKeyDialog(null)} disabled={codexApiKeyBusy}>取消</button>
              <button
                className="btn-primary"
                disabled={codexApiKeyBusy || !codexApiKeyDialog.apiKey.trim()}
                onClick={async () => {
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
                }}
              >
                {codexApiKeyBusy ? "保存中..." : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}

      {ideLabelDialog && (
        <div className="accounts-modal-overlay" onClick={() => !ideLabelBusy && setIdeLabelDialog(null)}>
          <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="accounts-modal-title">编辑账号备注名</h3>
            <p className="accounts-modal-desc">{ideLabelDialog.account.email}</p>
            <div className="accounts-form-group">
              <label className="accounts-form-label">备注名</label>
              <input
                className="accounts-form-input"
                placeholder="留空则恢复显示邮箱"
                value={ideLabelDialog.label}
                onChange={(e) => setIdeLabelDialog((prev) => prev ? { ...prev, label: e.target.value } : prev)}
                disabled={ideLabelBusy}
              />
            </div>
            <div className="accounts-modal-actions">
              <button className="btn-outline" onClick={() => setIdeLabelDialog(null)} disabled={ideLabelBusy}>取消</button>
              <button
                className="btn-primary"
                disabled={ideLabelBusy}
                onClick={async () => {
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
                }}
              >
                {ideLabelBusy ? "保存中..." : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function formatGeminiQuotaSummary(quotaJson?: string) {
  if (!quotaJson) return "—";
  try {
    const value = JSON.parse(quotaJson);
    const root = value?.quota ?? value;
    const buckets = Array.isArray(root?.buckets) ? root.buckets : [];
    let minPercent: number | null = null;
    for (const bucket of buckets) {
      const fraction = typeof bucket?.remainingFraction === "number"
        ? bucket.remainingFraction
        : typeof bucket?.remainingFraction === "string"
          ? Number(bucket.remainingFraction)
          : NaN;
      if (Number.isFinite(fraction)) {
        const percent = Math.max(0, Math.min(100, Math.round(fraction * 100)));
        minPercent = minPercent == null ? percent : Math.min(minPercent, percent);
      }
    }
    const projectId = typeof value?.project_id === "string" ? value.project_id : null;
    if (minPercent == null) return projectId ? `项目:${projectId}` : "已同步";
    return projectId ? `${minPercent}% · ${projectId}` : `${minPercent}%`;
  } catch {
    return "已同步";
  }
}

function formatIdePlatformLabel(account: IdeAccount) {
  if (account.origin_platform === "gemini") {
    return account.project_id ? `gemini · ${account.project_id}` : "gemini";
  }
  if (account.origin_platform === "codex") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.account_name || meta.plan_type || meta.auth_mode;
    return extra ? `codex · ${extra}` : "codex";
  }
  if (account.origin_platform === "cursor") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.membership_type || meta.subscription_status;
    return extra ? `cursor · ${extra}` : "cursor";
  }
  if (account.origin_platform === "windsurf") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.plan || meta.user_id;
    return extra ? `windsurf · ${extra}` : "windsurf";
  }
  if (account.origin_platform === "kiro") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `kiro · ${extra}` : "kiro";
  }
  if (account.origin_platform === "qoder") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `qoder · ${extra}` : "qoder";
  }
  if (account.origin_platform === "trae") {
    const meta = parseIdeMeta(account.meta_json);
    const extra = meta.user_id || account.label;
    return extra ? `trae · ${extra}` : "trae";
  }
  return account.origin_platform;
}

function isCodexApiKeyAccount(account: IdeAccount) {
  if (account.origin_platform !== "codex") return false;
  const meta = parseIdeMeta(account.meta_json);
  return meta.auth_mode === "apikey";
}

function isCurrentIdeAccount(account: IdeAccount, currentMap: Record<string, string | null>) {
  const currentId = currentMap[account.origin_platform];
  return !!currentId && currentId === account.id;
}

function getCurrentActionLabel(account: IdeAccount) {
  switch (account.origin_platform) {
    case "codex":
      return "设为当前 Codex 账号";
    case "gemini":
      return "设为当前 Gemini 账号";
    case "cursor":
      return "设为当前 Cursor 账号";
    case "windsurf":
      return "设为当前 Windsurf 账号";
    default:
      return "设为当前账号";
  }
}

function formatGeminiQuotaTooltip(quotaJson?: string) {
  if (!quotaJson) return "";
  try {
    const value = JSON.parse(quotaJson);
    const root = value?.quota ?? value;
    const buckets = Array.isArray(root?.buckets) ? root.buckets : [];
    const projectId = typeof value?.project_id === "string" ? value.project_id : "";
    const modelSummaries = buckets
      .map((bucket: any) => {
        const modelId = typeof bucket?.modelId === "string" ? bucket.modelId : "unknown";
        const fraction = typeof bucket?.remainingFraction === "number"
          ? bucket.remainingFraction
          : typeof bucket?.remainingFraction === "string"
            ? Number(bucket.remainingFraction)
            : NaN;
        if (!Number.isFinite(fraction)) return null;
        return `${modelId}: ${Math.max(0, Math.min(100, Math.round(fraction * 100)))}%`;
      })
      .filter(Boolean)
      .slice(0, 4);

    return [projectId ? `Project: ${projectId}` : null, ...modelSummaries]
      .filter(Boolean)
      .join("\n");
  } catch {
    return "";
  }
}

function formatCursorSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return meta.membership_type || meta.subscription_status || "已同步";
}

function formatCursorTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.auth_id === "string" ? `Auth ID: ${meta.auth_id}` : null,
    typeof meta.membership_type === "string" ? `Membership: ${meta.membership_type}` : null,
    typeof meta.subscription_status === "string" ? `Subscription: ${meta.subscription_status}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatWindsurfSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return meta.plan || "已同步";
}

function formatWindsurfTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    typeof meta.plan === "string" ? `Plan: ${meta.plan}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatKiroSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

function formatKiroTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatQoderSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

function formatQoderTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatTraeSummary(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return account.label || meta.user_id || "已同步";
}

function formatTraeTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  return [
    typeof meta.user_id === "string" ? `User ID: ${meta.user_id}` : null,
    account.label ? `Profile: ${account.label}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function parseIdeMeta(metaJson?: string) {
  if (!metaJson) return {} as Record<string, string>;
  try {
    const value = JSON.parse(metaJson);
    return typeof value === "object" && value ? value as Record<string, string> : {};
  } catch {
    return {} as Record<string, string>;
  }
}

function formatCodexQuotaSummary(quotaJson?: string) {
  if (!quotaJson) return "—";
  try {
    const value = JSON.parse(quotaJson);
    const hourly = typeof value?.hourly_percentage === "number" ? value.hourly_percentage : null;
    const weekly = typeof value?.weekly_percentage === "number" ? value.weekly_percentage : null;
    const planType = typeof value?.plan_type === "string" ? value.plan_type : null;
    if (hourly != null && weekly != null) return `${hourly}% / ${weekly}%`;
    if (hourly != null) return `${hourly}%`;
    if (weekly != null) return `周 ${weekly}%`;
    return planType || "已同步";
  } catch {
    return "已同步";
  }
}

function formatCodexQuotaTooltip(quotaJson?: string) {
  if (!quotaJson) return "";
  try {
    const value = JSON.parse(quotaJson);
    return [
      typeof value?.plan_type === "string" ? `Plan: ${value.plan_type}` : null,
      typeof value?.hourly_percentage === "number" ? `5h: ${value.hourly_percentage}%` : null,
      typeof value?.weekly_percentage === "number" ? `Weekly: ${value.weekly_percentage}%` : null,
    ]
      .filter(Boolean)
      .join("\n");
  } catch {
    return "";
  }
}

function formatCodexApiKeyTooltip(account: IdeAccount) {
  const meta = parseIdeMeta(account.meta_json);
  if (meta.auth_mode !== "apikey") return "";
  return [
    "Auth: API Key",
    typeof meta.api_base_url === "string" && meta.api_base_url ? `Base URL: ${meta.api_base_url}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

// 辅助子组件：状态渲染器
function StatusCell({ item }: { item: UnifiedAccountItem }) {
  if (item.type === "api") {
    const st = item.data.status;
    let cls = "unknown", text = STATUS_LABELS[st] || st;
    if (st === "valid") cls = "valid";
    else if (st === "banned" || st === "invalid" || st === "expired") cls = "invalid";
    return <span className={`status-badge ${cls}`}>{text}</span>;
  } else {
    const st = item.data.status;
    let cls = "unknown", text = st.toUpperCase();
    if (st === "active") cls = "valid";
    else if (st === "forbidden") cls = "invalid";
    else if (st === "rate_limited" || (st as any) === "rate_limit") cls = "warning";
    return <span className={`status-badge ${cls}`}>{text}</span>;
  }
}
