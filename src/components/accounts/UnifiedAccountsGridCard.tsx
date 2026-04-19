import { CalendarCheck, Edit2, Folder, Key, MonitorPlay, RefreshCw, Server, Share, ShieldCheck, X } from "lucide-react";
import { PLATFORM_LABELS } from "../../types";
import type { IdeAccount } from "../../types";
import UnifiedAccountBalanceCell from "./UnifiedAccountBalanceCell";
import UnifiedAccountStatusCell from "./UnifiedAccountStatusCell";
import { supportsDailyCheckin } from "./platformStatusActionUtils";
import {
  formatIdePlatformLabel,
  getCurrentActionLabel,
  getIdeRefreshActionLabel,
  isCurrentIdeAccount,
  isIdeRefreshSupported,
} from "./unifiedAccountsUtils";
import type { UnifiedAccountItem } from "./unifiedAccountsTypes";

export function UnifiedAccountsGridCard({
  item,
  itemKey,
  privacy,
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
  onSetCurrentIdeAccount,
  onRefreshIdeAccount,
  onOpenGeminiProject,
  onOpenCodexApiKey,
  onOpenIdeLabel,
  onDeleteIdeAccount,
}: {
  item: UnifiedAccountItem;
  itemKey: string;
  privacy: boolean;
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
  onSetCurrentIdeAccount: (account: IdeAccount) => void;
  onRefreshIdeAccount: (account: IdeAccount) => void;
  onOpenGeminiProject: (account: IdeAccount) => void;
  onOpenCodexApiKey: (account: IdeAccount) => void;
  onOpenIdeLabel: (account: IdeAccount) => void;
  onDeleteIdeAccount: (account: IdeAccount) => void;
}) {
  const isCurrent = item.type === "ide" ? isCurrentIdeAccount(item.data, currentIdeAccountIds) : false;
  const groupName = item.type === "ide" ? accountGroupByAccountId.get(item.data.id)?.name : null;
  const isIdeRefreshable = item.type === "ide" && isIdeRefreshSupported(item.data);

  return (
    <article key={itemKey} className="account-grid-card">
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
        <span>额度: <UnifiedAccountBalanceCell item={item} privacy={privacy} /></span>
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
            onChange={() => onToggleIdeSelected(item.data.id)}
          />
        )}
        <button className="btn-row-action" title="快速生成分享 Token" onClick={() => onCreateShareToken(item)}>
          <Share size={14} />
        </button>
        {item.type === "ide" && (
          <>
            {isCurrent ? (
              <button className="btn-row-action" disabled title="当前账号">
                <MonitorPlay size={14} />
              </button>
            ) : (
              <button
                className="btn-row-action"
                title={getCurrentActionLabel(item.data)}
                onClick={() => onSetCurrentIdeAccount(item.data)}
              >
                <MonitorPlay size={14} />
              </button>
            )}
            {supportsDailyCheckin(item.data) && (
              <button
                className="btn-row-action"
                title="执行每日签到（失败自动重试 1 次）"
                onClick={() => onRunDailyCheckin(item.data)}
                disabled={isStatusActionPending}
              >
                <CalendarCheck size={14} />
              </button>
            )}
            {isIdeRefreshable && (
              <button
                className="btn-row-action"
                onClick={() => onRefreshIdeAccount(item.data)}
                title={getIdeRefreshActionLabel(item.data.origin_platform)}
              >
                <RefreshCw size={14} />
              </button>
            )}
            {item.data.origin_platform === "gemini" && (
              <button className="btn-row-action" onClick={() => onOpenGeminiProject(item.data)} title="设置 Gemini 项目">
                <Folder size={14} />
              </button>
            )}
            {item.data.origin_platform === "codex" && (
              <button className="btn-row-action" onClick={() => onOpenCodexApiKey(item.data)} title="编辑 Codex API Key 凭证">
                <Key size={14} />
              </button>
            )}
            <button className="btn-row-action" onClick={() => onOpenIdeLabel(item.data)} title="编辑备注名">
              <Edit2 size={14} />
            </button>
            <button className="btn-row-action danger" onClick={() => onDeleteIdeAccount(item.data)} title="拔除资产">
              <X size={14} />
            </button>
          </>
        )}
        {item.type === "api" && (
          <>
            <button className="btn-row-action" onClick={() => onRefreshApiBalance(item.data.id)} title="刷新余额">
              <RefreshCw size={14} />
            </button>
            <button className="btn-row-action" onClick={() => onCheckApiKey(item.data.id)} title="探测连通性">
              <MonitorPlay size={14} />
            </button>
          </>
        )}
      </div>
    </article>
  );
}
