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
  Search, Eye, EyeOff, RefreshCw, Plus, X, MonitorPlay, Share
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

  const isLoading = keysLoading || ideLoading;

  // ---------- Mutations ----------
  const deleteKeyMut = useMutation({ mutationFn: api.keys.delete, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const checkKeyMut = useMutation({ mutationFn: api.keys.check, onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }) });
  const refreshBalMut = useMutation({ mutationFn: (id: string) => api.balance.refreshOne(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }) });
  const deleteIdeMut = useMutation({ mutationFn: api.ideAccounts.delete, onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }) });
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

  const activeChannelName = channels.all.id === activeChannelId ? "系统全局资产" : 
                            [...channels.apiChs, ...channels.ideChs].find(c => c.id === activeChannelId)?.label || "未知渠道";

  const totalFilteredCount = displayItems.length;

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
                          : (item.type === "api" ? item.data.name : item.data.email)
                        }
                      </span>
                    </div>

                    {/* Platform */}
                    <div className="col-platform table-cell-ellipsis text-muted">
                      {item.type === "api" ? (PLATFORM_LABELS[item.data.platform as keyof typeof PLATFORM_LABELS] || item.data.platform) : item.data.origin_platform}
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
                        if(confirm(`是否为 ${item.type==='api'?item.data.name:item.data.email} 单独签发透传 Token？`)){
                          try {
                            await api.userTokens.create({
                              username: `[极速生成] ${item.type==='api'?item.data.name:item.data.email}`,
                              description: JSON.stringify({ desc: "单点直连专用", scope: "single", single_account: item.data.id }),
                              expires_type: "never", expires_at: null, max_ips: 0, curfew_start: null, curfew_end: null
                            });
                            alert(`已为您生成底座专属直连 Token，请切换至【分享额度】页面进行查看和提取！`);
                          }catch(e){ alert("生成失败: "+e); }
                        }
                      }} title="快速生成分享 Token"><Share size={14}/></button>

                      {item.type === "api" ? (
                        <>
                          <button className="btn-row-action" onClick={() => refreshBalMut.mutate(item.data.id)} title="刷新余额"><RefreshCw size={14}/></button>
                          <button className="btn-row-action" onClick={() => checkKeyMut.mutate(item.data.id)} title="探测连通性"><MonitorPlay size={14}/></button>
                          <button className="btn-row-action danger" onClick={() => { if(confirm("删除密钥？")) deleteKeyMut.mutate(item.data.id); }} title="彻底销毁"><X size={14}/></button>
                        </>
                      ) : (
                        <>
                          <button className="btn-row-action" onClick={async () => {
                            if(confirm(`强制下发配置 ${item.data.email}？`)){
                              api.ideAccounts.forceInject(item.data.id).then(()=>alert("注射成功")).catch(e=>alert(e));
                            }
                          }} title="作为全局底层配置注射"><MonitorPlay size={14}/></button>
                          <button className="btn-row-action danger" onClick={() => { if(confirm("删除指纹？")) deleteIdeMut.mutate(item.data.id); }} title="拔除资产"><X size={14}/></button>
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
    </div>
  );
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
