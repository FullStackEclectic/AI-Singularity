import { History } from "lucide-react";
import type { AccountGroup } from "../../types";
import type { WakeupHistoryGroup } from "./wakeupUtils";
import {
  getWakeupCategoryTone,
  WAKEUP_UNGROUPED_FILTER,
} from "./wakeupUtils";

type WakeupHistoryPanelProps = {
  historyCount: number;
  historyGroupFilter: string;
  historyGroupOptions: AccountGroup[];
  hasUngroupedHistory: boolean;
  groupedHistory: WakeupHistoryGroup[];
  getAccountGroupLabel: (accountId?: string) => string;
  getWakeupCategoryLabel: (category?: string | null) => string;
  onHistoryGroupFilterChange: (value: string) => void;
  onClearHistory: () => void;
};

export function WakeupHistoryPanel({
  historyCount,
  historyGroupFilter,
  historyGroupOptions,
  hasUngroupedHistory,
  groupedHistory,
  getAccountGroupLabel,
  getWakeupCategoryLabel,
  onHistoryGroupFilterChange,
  onClearHistory,
}: WakeupHistoryPanelProps) {
  return (
    <section className="card wakeup-history-panel">
      <div className="wakeup-history-header">
        <div className="wakeup-section-title">
          <History size={16} /> 执行历史
        </div>
        <button className="btn btn-danger-ghost btn-sm" onClick={onClearHistory}>
          清空历史
        </button>
      </div>

      <div className="wakeup-group-filters">
        <button
          className={`btn btn-secondary btn-sm ${historyGroupFilter === "all" ? "active" : ""}`}
          onClick={() => onHistoryGroupFilterChange("all")}
          type="button"
        >
          全部分组
        </button>
        {hasUngroupedHistory && (
          <button
            className={`btn btn-secondary btn-sm ${historyGroupFilter === WAKEUP_UNGROUPED_FILTER ? "active" : ""}`}
            onClick={() => onHistoryGroupFilterChange(WAKEUP_UNGROUPED_FILTER)}
            type="button"
          >
            未分组
          </button>
        )}
        {historyGroupOptions.map((group) => (
          <button
            key={group.id}
            className={`btn btn-secondary btn-sm ${historyGroupFilter === group.id ? "active" : ""}`}
            onClick={() => onHistoryGroupFilterChange(group.id)}
            type="button"
          >
            {group.name}
          </button>
        ))}
      </div>

      {historyCount === 0 ? (
        <div className="wakeup-empty">当前还没有 Wakeup 历史</div>
      ) : groupedHistory.length === 0 ? (
        <div className="wakeup-empty">当前分组下没有 Wakeup 历史</div>
      ) : (
        <div className="wakeup-history-list">
          {groupedHistory.map((group) => (
            <div key={group.runId} className="wakeup-history-item">
              <div className="wakeup-history-top">
                <strong>{group.latest.task_name}</strong>
                <span>{new Date(group.latest.created_at).toLocaleString()}</span>
              </div>
              <div className="wakeup-history-meta">
                <span>本批次 {group.items.length} 条</span>
                <span>成功：{group.successCount}</span>
                <span>失败：{group.failedCount}</span>
                <span>模型：{group.latest.model || "—"}</span>
              </div>
              <div className="wakeup-history-list" style={{ marginTop: 10 }}>
                {group.items.map((item) => (
                  <div key={item.id} className="wakeup-history-item wakeup-history-subitem">
                    <div className="wakeup-history-top">
                      <strong>{item.account_id || "—"}</strong>
                      <span>{item.status}</span>
                    </div>
                    <div className="wakeup-history-meta">
                      <span>分类：{getWakeupCategoryLabel(item.category)}</span>
                      <span className={`wakeup-category-badge ${getWakeupCategoryTone(item.category)}`}>
                        {getWakeupCategoryLabel(item.category)}
                      </span>
                      <span>分组：{getAccountGroupLabel(item.account_id)}</span>
                    </div>
                    {item.message && <div className="wakeup-history-message">{item.message}</div>}
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
