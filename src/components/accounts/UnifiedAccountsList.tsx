import { useState, useMemo, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { PLATFORM_LABELS, STATUS_LABELS } from "../../types";
import type { ApiKey, IdeAccount, Balance } from "../../types";
import AddAccountWizard from "./AddAccountWizard";
import GroupManagerModal from "./GroupManagerModal";
import { TagBadgeList } from "./TagEditorPopover";
import { getGroups, type AccountGroup } from "../../lib/groupService";
import { isPrivacyMode, setPrivacyMode, maskEmail, maskToken } from "../../lib/privacyMode";
import "./UnifiedAccountsList.css";

// ─── 工具函数 ──────────────────────────────────────────────────────────────────

function getTagFiltersFromAccounts(ideAccs: IdeAccount[], keys: ApiKey[]): string[] {
  const all = new Set<string>();
  ideAccs.forEach((a) => (a.tags ?? []).forEach((t) => all.add(t)));
  keys.forEach((k) => (k.tags ?? []).forEach((t) => all.add(t)));
  return Array.from(all).sort();
}

// ─── 主组件 ────────────────────────────────────────────────────────────────────

export default function UnifiedAccountsList() {
  const qc = useQueryClient();

  // ---------- UI State ----------
  const [showAddWizard, setShowAddWizard] = useState(false);
  const [showGroupManager, setShowGroupManager] = useState(false);
  const [privacy, setPrivacy] = useState(isPrivacyMode);

  // 过滤状态
  const [searchQuery, setSearchQuery] = useState("");
  const [activeGroupId, setActiveGroupId] = useState<string | null>(null); // null = 全部
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());

  // 分组数据（由 GroupManagerModal 刷新触发）
  const [groups, setGroups] = useState<AccountGroup[]>(() => getGroups());

  const handleGroupsChanged = useCallback(() => {
    setGroups(getGroups());
  }, []);

  // ---------- Data ----------
  const { data: keys = [], isLoading: keysLoading } = useQuery({
    queryKey: ["keys"],
    queryFn: api.keys.list,
  });

  const { data: balances = [] } = useQuery({
    queryKey: ["balances"],
    queryFn: api.balance.listAll,
    staleTime: 1000 * 60 * 5,
  });
  const balanceMap = Object.fromEntries(balances.map((b) => [b.key_id, b]));

  const { data: ideAccs = [], isLoading: ideLoading } = useQuery({
    queryKey: ["ideAccounts"],
    queryFn: api.ideAccounts.list,
  });

  const isLoading = keysLoading || ideLoading;

  // ---------- Mutations ----------
  const deleteKeyMut = useMutation({
    mutationFn: api.keys.delete,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });
  const checkKeyMut = useMutation({
    mutationFn: api.keys.check,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });
  const refreshBalanceMut = useMutation({
    mutationFn: (id: string) => api.balance.refreshOne(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }),
  });
  const deleteIdeMut = useMutation({
    mutationFn: api.ideAccounts.delete,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["ideAccounts"] }),
  });

  // ---------- Filtering ----------
  const allTagOptions = useMemo(() => getTagFiltersFromAccounts(ideAccs, keys), [ideAccs, keys]);

  const filteredIdeAccs = useMemo(() => {
    let list = ideAccs;

    // 分组过滤
    if (activeGroupId) {
      const groupAccIds = groups.find((g) => g.id === activeGroupId)?.accountIds ?? [];
      list = list.filter((a) => groupAccIds.includes(a.id));
    }

    // 标签过滤
    if (selectedTags.size > 0) {
      list = list.filter((a) => (a.tags ?? []).some((t) => selectedTags.has(t)));
    }

    // 搜索过滤
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(
        (a) =>
          a.email.toLowerCase().includes(q) ||
          a.origin_platform.toLowerCase().includes(q) ||
          (a.tags ?? []).some((t) => t.toLowerCase().includes(q))
      );
    }
    return list;
  }, [ideAccs, activeGroupId, selectedTags, searchQuery, groups]);

  const filteredKeys = useMemo(() => {
    let list = keys;

    if (activeGroupId) {
      const groupAccIds = groups.find((g) => g.id === activeGroupId)?.accountIds ?? [];
      list = list.filter((k) => groupAccIds.includes(k.id));
    }

    if (selectedTags.size > 0) {
      list = list.filter((k) => (k.tags ?? []).some((t) => selectedTags.has(t)));
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(
        (k) =>
          k.name.toLowerCase().includes(q) ||
          k.platform.toLowerCase().includes(q) ||
          (k.tags ?? []).some((t) => t.toLowerCase().includes(q))
      );
    }
    return list;
  }, [keys, activeGroupId, selectedTags, searchQuery, groups]);

  const noAccounts = keys.length === 0 && ideAccs.length === 0;
  const validCount =
    keys.filter((k) => k.status === "valid").length +
    ideAccs.filter((a) => a.status === "active").length;

  const toggleTag = (tag: string) => {
    setSelectedTags((prev) => {
      const next = new Set(prev);
      next.has(tag) ? next.delete(tag) : next.add(tag);
      return next;
    });
  };

  const togglePrivacy = () => {
    const next = !privacy;
    setPrivacy(next);
    setPrivacyMode(next);
  };

  return (
    <div className="unified-accounts-page">
      {/* ─── Header ─────────────────────────────────────────────────── */}
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span style={{ color: "var(--accent-primary)" }}>🔌</span> 接入账号
          </h1>
          <p className="page-subtitle">
            管理正在为系统提供底层算力的各类账号及凭证，包括标准 API 以及 IDE 辅助插件账户。
          </p>
        </div>
        <div className="header-actions">
          <button
            className={`btn btn-icon-label ${privacy ? "active" : ""}`}
            onClick={togglePrivacy}
            title={privacy ? "关闭隐私模式" : "开启隐私模式"}
          >
            {privacy ? "🙈" : "👁️"}
            <span>{privacy ? "隐私中" : "隐私"}</span>
          </button>
          <button
            className="btn btn-outline"
            onClick={() => setShowGroupManager(true)}
            title="管理分组"
          >
            📁 分组
          </button>
          <button
            className="btn btn-outline"
            onClick={() => keys.forEach((k) => checkKeyMut.mutate(k.id))}
            disabled={keys.length === 0 || checkKeyMut.isPending}
          >
            ⟳ 探测
          </button>
          <button className="btn btn-primary" onClick={() => setShowAddWizard(true)}>
            ＋ 添加账号
          </button>
        </div>
      </div>

      {/* ─── Stats Bar ───────────────────────────────────────────────── */}
      <div className="stats-bar">
        <div className="stat-item">
          <span className="label">接入总数</span>
          <span className="value">{keys.length + ideAccs.length}</span>
        </div>
        <div className="stat-item">
          <span className="label text-success">正常运作</span>
          <span className="value text-success">{validCount}</span>
        </div>
        <div className="stat-item">
          <span className="label">标准 API</span>
          <span className="value">{keys.length}</span>
        </div>
        <div className="stat-item">
          <span className="label">IDE 账号</span>
          <span className="value">{ideAccs.length}</span>
        </div>
        <div className="stat-item">
          <span className="label">分组</span>
          <span className="value">{groups.length}</span>
        </div>
      </div>

      {/* ─── Filter Bar ──────────────────────────────────────────────── */}
      <div className="filter-bar">
        {/* 搜索框 */}
        <div className="search-box">
          <span className="search-icon">🔍</span>
          <input
            className="search-input"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索账号..."
          />
          {searchQuery && (
            <button className="search-clear" onClick={() => setSearchQuery("")}>✕</button>
          )}
        </div>

        {/* 分组过滤 Tab */}
        <div className="group-tabs">
          <button
            className={`group-tab ${activeGroupId === null ? "active" : ""}`}
            onClick={() => setActiveGroupId(null)}
          >
            全部 <span className="tab-count">{keys.length + ideAccs.length}</span>
          </button>
          {groups.map((g) => (
            <button
              key={g.id}
              className={`group-tab ${activeGroupId === g.id ? "active" : ""}`}
              onClick={() => setActiveGroupId(g.id === activeGroupId ? null : g.id)}
            >
              {g.name}
              <span className="tab-count">{g.accountIds.length}</span>
            </button>
          ))}
        </div>

        {/* 标签过滤 */}
        {allTagOptions.length > 0 && (
          <div className="tag-filter-bar">
            {allTagOptions.map((tag) => (
              <button
                key={tag}
                className={`tag-filter-chip ${selectedTags.has(tag) ? "selected" : ""}`}
                onClick={() => toggleTag(tag)}
              >
                {tag}
              </button>
            ))}
            {selectedTags.size > 0 && (
              <button
                className="tag-filter-clear"
                onClick={() => setSelectedTags(new Set())}
              >
                清除过滤
              </button>
            )}
          </div>
        )}
      </div>

      {/* ─── Account List ────────────────────────────────────────────── */}
      {isLoading ? (
        <div className="empty-state">
          <div className="animate-spin" style={{ fontSize: 24, marginBottom: "1rem" }}>⟳</div>
          <span>加载中...</span>
        </div>
      ) : noAccounts ? (
        <div className="empty-state">
          <h3 style={{ margin: "0 0 0.5rem 0", color: "var(--text-primary)" }}>暂无接入账号</h3>
          <p style={{ margin: "0 0 1.5rem 0" }}>没有探测到任何可用的算力凭证，系统能力受限。</p>
          <button className="btn btn-primary" onClick={() => setShowAddWizard(true)}>
            立即添加账号
          </button>
        </div>
      ) : filteredIdeAccs.length === 0 && filteredKeys.length === 0 ? (
        <div className="empty-state">
          <p style={{ margin: 0, color: "var(--text-muted)" }}>没有匹配的账号，请调整过滤条件。</p>
        </div>
      ) : (
        <div className="accounts-grid">
          {/* Standard API Keys */}
          {filteredKeys.map((key) => (
            <StandardKeyCard
              key={key.id}
              apiKey={key}
              balance={balanceMap[key.id]}
              onCheck={() => checkKeyMut.mutate(key.id)}
              onDelete={() => deleteKeyMut.mutate(key.id)}
              onRefreshBalance={() => refreshBalanceMut.mutate(key.id)}
              isChecking={checkKeyMut.isPending}
              privacy={privacy}
              groups={groups.filter((g) => g.accountIds.includes(key.id))}
            />
          ))}

          {/* IDE Fingerprint Accounts */}
          {filteredIdeAccs.map((acc) => (
            <IdeAccountCard
              key={acc.id}
              account={acc}
              onDelete={() => deleteIdeMut.mutate(acc.id)}
              privacy={privacy}
              groups={groups.filter((g) => g.accountIds.includes(acc.id))}
            />
          ))}
        </div>
      )}

      {/* ─── Modals ───────────────────────────────────────────────────── */}
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

      {showGroupManager && (
        <GroupManagerModal
          ideAccounts={ideAccs}
          apiKeys={keys}
          onClose={() => setShowGroupManager(false)}
          onGroupsChanged={handleGroupsChanged}
        />
      )}
    </div>
  );
}

// ─── Standard API Key Card ────────────────────────────────────────────────────

function StandardKeyCard({
  apiKey,
  balance,
  onCheck,
  onDelete,
  onRefreshBalance,
  isChecking,
  privacy,
  groups,
}: {
  apiKey: ApiKey;
  balance?: Balance;
  onCheck: () => void;
  onDelete: () => void;
  onRefreshBalance: () => void;
  isChecking: boolean;
  privacy: boolean;
  groups: AccountGroup[];
}) {
  const statusClass =
    apiKey.status === "valid"
      ? "valid"
      : apiKey.status === "banned" || apiKey.status === "invalid" || apiKey.status === "expired"
      ? "invalid"
      : isChecking
      ? "checking"
      : "unknown";

  const balanceText = (() => {
    if (!balance) return null;
    if (balance.balance_usd != null) return `$${balance.balance_usd.toFixed(2)}`;
    if (balance.balance_cny != null) return `¥${balance.balance_cny.toFixed(2)}`;
    return null;
  })();

  return (
    <div className="account-card">
      <div className="account-card-header">
        <div>
          <div className="card-title-row">
            <span className={`status-dot ${statusClass}`} />
            <span className="name">{apiKey.name}</span>
            <span className="card-type-badge text-muted">标准接口</span>
          </div>
          <div className="text-muted" style={{ fontSize: "0.8rem" }}>
            {PLATFORM_LABELS[apiKey.platform] || apiKey.platform}
          </div>
        </div>
      </div>

      <div className="account-card-body">
        <div className="preview-code">
          {privacy ? maskToken(apiKey.key_preview) : apiKey.key_preview}
        </div>

        {apiKey.base_url && (
          <div className="card-row">
            <span className="label">接口地址</span>
            <span>{apiKey.base_url}</span>
          </div>
        )}

        <div className="card-row">
          <span className="label">状态</span>
          <span
            className={`text-${
              statusClass === "valid"
                ? "success"
                : statusClass === "invalid"
                ? "danger"
                : "muted"
            }`}
          >
            {isChecking ? "检测中..." : STATUS_LABELS[apiKey.status] || "未知"}
          </span>
        </div>

        {balanceText && (
          <div className="card-row">
            <span className="label">账户余额</span>
            <span style={{ display: "flex", gap: "8px", alignItems: "center" }}>
              {privacy ? "***" : balanceText}
              <button className="btn-icon" onClick={onRefreshBalance} style={{ padding: 0 }} title="刷新余额">
                ⟳
              </button>
            </span>
          </div>
        )}

        {/* 标签 */}
        <div className="card-row" style={{ alignItems: "flex-start" }}>
          <span className="label">标签</span>
          <TagBadgeList tags={apiKey.tags} accountId={apiKey.id} accountType="api" />
        </div>

        {/* 分组 */}
        {groups.length > 0 && (
          <div className="card-row">
            <span className="label">分组</span>
            <span className="groups-inline">
              {groups.map((g) => (
                <span key={g.id} className="group-badge">{g.name}</span>
              ))}
            </span>
          </div>
        )}
      </div>

      <div className="account-card-footer">
        <div className="priority-info text-muted" style={{ fontSize: "0.8rem" }}>
          优先级: {apiKey.priority ?? 100}
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <button className="btn-icon" onClick={onCheck} title="连通性测试">⟳</button>
          <button className="btn-icon danger" onClick={onDelete} title="删除">✕</button>
        </div>
      </div>
    </div>
  );
}

// ─── IDE Fingerprint Account Card ────────────────────────────────────────────

function IdeAccountCard({
  account,
  onDelete,
  privacy,
  groups,
}: {
  account: IdeAccount;
  onDelete: () => void;
  privacy: boolean;
  groups: AccountGroup[];
}) {
  const statusClass =
    account.status === "active"
      ? "active"
      : account.status === "forbidden"
      ? "invalid"
      : "rate_limited";

  const displayEmail = privacy ? maskEmail(account.email) : account.email;

  return (
    <div className="account-card">
      <div className="account-card-header">
        <div>
          <div className="card-title-row">
            <span className={`status-dot ${statusClass}`} />
            <span className="name">{displayEmail}</span>
            <span className="card-type-badge text-muted">插件账号</span>
          </div>
          <div className="text-muted" style={{ fontSize: "0.8rem" }}>
            {account.origin_platform} 环境池
          </div>
        </div>
      </div>

      <div className="account-card-body">
        <div className="card-row">
          <span className="label">状态</span>
          <span
            className={`text-${
              statusClass === "active"
                ? "success"
                : statusClass === "invalid"
                ? "danger"
                : "warning"
            }`}
          >
            {account.status.toUpperCase()}
          </span>
        </div>

        {account.disabled_reason && (
          <div className="card-row">
            <span className="label">不可用原因</span>
            <span className="text-danger" style={{ maxWidth: "180px", textAlign: "right" }}>
              {account.disabled_reason}
            </span>
          </div>
        )}

        <div className="card-row">
          <span className="label">设备指纹</span>
          <span>{account.device_profile ? "已启用" : "未挂载"}</span>
        </div>

        {/* 标签 */}
        <div className="card-row" style={{ alignItems: "flex-start" }}>
          <span className="label">标签</span>
          <TagBadgeList tags={account.tags} accountId={account.id} accountType="ide" />
        </div>

        {/* 分组 */}
        {groups.length > 0 && (
          <div className="card-row">
            <span className="label">分组</span>
            <span className="groups-inline">
              {groups.map((g) => (
                <span key={g.id} className="group-badge">{g.name}</span>
              ))}
            </span>
          </div>
        )}
      </div>

      <div className="account-card-footer">
        <div className="text-muted" style={{ fontSize: "0.8rem" }}>
          心跳: {new Date(account.last_used).toLocaleString()}
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            className="btn btn-outline"
            style={{ padding: "2px 8px", fontSize: "11px" }}
            onClick={async () => {
              if (window.confirm(`确认要强制配置此账号 (${account.email}) 到本地吗？`)) {
                try {
                  await api.ideAccounts.forceInject(account.id);
                  alert("替换完成！IDE配置已被更新。");
                } catch (e: any) {
                  alert("替换失败：" + e.toString());
                }
              }
            }}
            title="强行作为底层运行账户"
          >
            强制配置
          </button>
          <button className="btn-icon danger" onClick={onDelete} title="删除">
            ✕
          </button>
        </div>
      </div>
    </div>
  );
}
