import { Play } from "lucide-react";
import type { WakeupVerificationBatchResult } from "../../lib/api";
import type {
  AccountGroup,
  IdeAccount,
} from "../../types";
import { WAKEUP_UNGROUPED_FILTER } from "./wakeupUtils";

type WakeupVerificationPanelProps = {
  accounts: IdeAccount[];
  visibleAccounts: IdeAccount[];
  verificationGroupOptions: AccountGroup[];
  verificationSelection: string[];
  verificationModel: string;
  verificationPrompt: string;
  verificationCommandTemplate: string;
  verificationTimeout: number;
  verificationRetryFailedTimes: number;
  verificationRunning: boolean;
  verificationShowFailedOnly: boolean;
  verificationGroupFilter: string;
  verificationResultGroupFilter: string;
  verificationResult: WakeupVerificationBatchResult | null;
  verificationResultGroupOptions: AccountGroup[];
  verificationVisibleItems: WakeupVerificationBatchResult["items"];
  verificationFailedAccountIds: string[];
  activeVerificationRunId: string | null;
  hasUngroupedVerificationResult: boolean;
  getAccountGroupLabel: (accountId?: string) => string;
  getWakeupCategoryLabel: (category?: string | null) => string;
  onVerificationGroupFilterChange: (value: string) => void;
  onVerificationSelectionChange: (value: string[]) => void;
  onVerificationModelChange: (value: string) => void;
  onVerificationPromptChange: (value: string) => void;
  onVerificationCommandTemplateChange: (value: string) => void;
  onVerificationTimeoutChange: (value: number) => void;
  onVerificationRetryFailedTimesChange: (value: number) => void;
  onToggleVerificationAccount: (accountId: string) => void;
  onRunVerification: () => void;
  onCancelVerification: () => void;
  onVerificationShowFailedOnlyChange: (value: boolean) => void;
  onVerificationResultGroupFilterChange: (value: string) => void;
  onRerunFailedVerification: () => void;
};

export function WakeupVerificationPanel({
  accounts,
  visibleAccounts,
  verificationGroupOptions,
  verificationSelection,
  verificationModel,
  verificationPrompt,
  verificationCommandTemplate,
  verificationTimeout,
  verificationRetryFailedTimes,
  verificationRunning,
  verificationShowFailedOnly,
  verificationGroupFilter,
  verificationResultGroupFilter,
  verificationResult,
  verificationResultGroupOptions,
  verificationVisibleItems,
  verificationFailedAccountIds,
  activeVerificationRunId,
  hasUngroupedVerificationResult,
  getAccountGroupLabel,
  getWakeupCategoryLabel,
  onVerificationGroupFilterChange,
  onVerificationSelectionChange,
  onVerificationModelChange,
  onVerificationPromptChange,
  onVerificationCommandTemplateChange,
  onVerificationTimeoutChange,
  onVerificationRetryFailedTimesChange,
  onToggleVerificationAccount,
  onRunVerification,
  onCancelVerification,
  onVerificationShowFailedOnlyChange,
  onVerificationResultGroupFilterChange,
  onRerunFailedVerification,
}: WakeupVerificationPanelProps) {
  return (
    <section className="card wakeup-history-panel">
      <div className="wakeup-history-header">
        <div className="wakeup-section-title">
          <Play size={16} /> 批次验证
        </div>
        <div className="wakeup-header-actions">
          <button className="btn btn-primary btn-sm" onClick={onRunVerification} disabled={verificationRunning}>
            {verificationRunning ? "执行中..." : "开始验证"}
          </button>
          <button
            className="btn btn-danger-ghost btn-sm"
            onClick={onCancelVerification}
            disabled={!verificationRunning || !activeVerificationRunId}
          >
            取消当前批次
          </button>
        </div>
      </div>

      <div className="wakeup-verification-config">
        <label>
          <span>验证模型</span>
          <input
            className="form-input"
            value={verificationModel}
            onChange={(event) => onVerificationModelChange(event.target.value)}
            placeholder="例如 gemini-2.5-pro"
          />
        </label>
        <label>
          <span>Prompt</span>
          <textarea
            className="form-input wakeup-textarea"
            value={verificationPrompt}
            onChange={(event) => onVerificationPromptChange(event.target.value)}
          />
        </label>
        <label>
          <span>命令模板</span>
          <input
            className="form-input"
            value={verificationCommandTemplate}
            onChange={(event) => onVerificationCommandTemplateChange(event.target.value)}
          />
        </label>
        <label>
          <span>超时秒数</span>
          <input
            className="form-input"
            type="number"
            min={10}
            max={3600}
            value={verificationTimeout}
            onChange={(event) => onVerificationTimeoutChange(Math.max(10, Number(event.target.value) || 120))}
          />
        </label>
        <label>
          <span>失败重试次数</span>
          <input
            className="form-input"
            type="number"
            min={0}
            max={5}
            value={verificationRetryFailedTimes}
            onChange={(event) =>
              onVerificationRetryFailedTimesChange(Math.max(0, Math.min(5, Number(event.target.value) || 0)))
            }
          />
        </label>
      </div>

      <div className="wakeup-verification-accounts">
        <div className="wakeup-group-filters">
          <button
            className={`btn btn-secondary btn-sm ${verificationGroupFilter === "all" ? "active" : ""}`}
            onClick={() => onVerificationGroupFilterChange("all")}
            type="button"
          >
            全部分组
          </button>
          <button
            className={`btn btn-secondary btn-sm ${verificationGroupFilter === WAKEUP_UNGROUPED_FILTER ? "active" : ""}`}
            onClick={() => onVerificationGroupFilterChange(WAKEUP_UNGROUPED_FILTER)}
            type="button"
          >
            未分组
          </button>
          {verificationGroupOptions.map((group) => (
            <button
              key={group.id}
              className={`btn btn-secondary btn-sm ${verificationGroupFilter === group.id ? "active" : ""}`}
              onClick={() => onVerificationGroupFilterChange(group.id)}
              type="button"
            >
              {group.name}
            </button>
          ))}
          <button
            className="btn btn-secondary btn-sm"
            type="button"
            disabled={visibleAccounts.length === 0}
            onClick={() =>
              onVerificationSelectionChange([
                ...new Set([...verificationSelection, ...visibleAccounts.map((account) => account.id)]),
              ])
            }
          >
            只选当前分组
          </button>
        </div>

        {accounts.length === 0 ? (
          <div className="wakeup-empty">当前没有可验证的 IDE 账号</div>
        ) : visibleAccounts.length === 0 ? (
          <div className="wakeup-empty">当前分组下没有可验证账号</div>
        ) : (
          visibleAccounts.map((account) => (
            <label key={account.id} className="wakeup-verification-account">
              <input
                type="checkbox"
                checked={verificationSelection.includes(account.id)}
                onChange={() => onToggleVerificationAccount(account.id)}
              />
              <span>{(account.label || account.email) + " · " + account.origin_platform + " · " + getAccountGroupLabel(account.id)}</span>
            </label>
          ))
        )}
      </div>

      {verificationResult && (
        <div className="wakeup-verification-result">
          <div className="wakeup-history-top">
            <strong>本次结果</strong>
            <span>执行 {verificationResult.executed_count} 个</span>
          </div>
          <div className="wakeup-history-meta">
            <span>成功：{verificationResult.success_count}</span>
            <span>失败：{verificationResult.failed_count}</span>
            <span>重试：{verificationResult.retried_count}</span>
          </div>
          <div className="wakeup-category-grid">
            {verificationResult.category_counts.map((item) => (
              <div key={item.category} className="wakeup-category-chip">
                <span>{getWakeupCategoryLabel(item.category)}</span>
                <strong>{item.count}</strong>
              </div>
            ))}
          </div>

          <div className="wakeup-history-header" style={{ marginTop: 8 }}>
            <div className="wakeup-history-meta">
              <span>结果列表</span>
              <span>当前显示：{verificationVisibleItems.length}</span>
            </div>
            <div className="wakeup-header-actions">
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => onVerificationShowFailedOnlyChange(!verificationShowFailedOnly)}
              >
                {verificationShowFailedOnly ? "显示全部" : "仅看失败"}
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={onRerunFailedVerification}
                disabled={verificationRunning || verificationFailedAccountIds.length === 0}
              >
                重试失败账号
              </button>
            </div>
          </div>

          <div className="wakeup-group-filters">
            <button
              className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === "all" ? "active" : ""}`}
              onClick={() => onVerificationResultGroupFilterChange("all")}
              type="button"
            >
              全部分组
            </button>
            {hasUngroupedVerificationResult && (
              <button
                className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === WAKEUP_UNGROUPED_FILTER ? "active" : ""}`}
                onClick={() => onVerificationResultGroupFilterChange(WAKEUP_UNGROUPED_FILTER)}
                type="button"
              >
                未分组
              </button>
            )}
            {verificationResultGroupOptions.map((group) => (
              <button
                key={group.id}
                className={`btn btn-secondary btn-sm ${verificationResultGroupFilter === group.id ? "active" : ""}`}
                onClick={() => onVerificationResultGroupFilterChange(group.id)}
                type="button"
              >
                {group.name}
              </button>
            ))}
          </div>

          <div className="wakeup-history-list">
            {verificationVisibleItems.length === 0 ? (
              <div className="wakeup-empty">当前分组下没有匹配的验证结果</div>
            ) : (
              verificationVisibleItems.map((item) => (
                <div key={item.account_id} className="wakeup-history-item">
                  <div className="wakeup-history-top">
                    <strong>{item.email}</strong>
                    <span>{item.status}</span>
                  </div>
                  <div className="wakeup-history-meta">
                    <span>分类：{getWakeupCategoryLabel(item.category)}</span>
                    <span>尝试：{item.attempts}</span>
                    <span>分组：{getAccountGroupLabel(item.account_id)}</span>
                  </div>
                  <div className="wakeup-history-message">{item.message}</div>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </section>
  );
}
