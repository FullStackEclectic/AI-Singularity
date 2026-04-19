import { Clock3, Save } from "lucide-react";
import type { WakeupState } from "../../lib/api";

type WakeupOverviewProps = {
  state: WakeupState;
  message: string;
  saving: boolean;
  activeTasksCount: number;
  historyCount: number;
  failingTasksCount: number;
  onAddTask: () => void;
  onSave: () => void;
  onStateChange: (next: WakeupState) => void;
};

export function WakeupOverview({
  state,
  message,
  saving,
  activeTasksCount,
  historyCount,
  failingTasksCount,
  onAddTask,
  onSave,
  onStateChange,
}: WakeupOverviewProps) {
  return (
    <>
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <Clock3 size={22} className="text-primary" /> Wakeup / Verification
          </h1>
          <p className="page-subtitle">
            任务、历史、cron 调度、即时执行和批次验证已接入。本页已支持结果分类统计与失败账号重试。
          </p>
        </div>
        <div className="wakeup-header-actions">
          <button className="btn btn-secondary" onClick={onAddTask}>
            新增任务
          </button>
          <button className="btn btn-primary" onClick={onSave} disabled={saving}>
            <Save size={14} /> {saving ? "保存中..." : "保存全部"}
          </button>
        </div>
      </div>

      {message && <div className="wakeup-message">{message}</div>}

      <div className="wakeup-overview">
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">总任务数</div>
          <div className="wakeup-stat-value">{state.tasks.length}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">启用任务</div>
          <div className="wakeup-stat-value">{activeTasksCount}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">历史记录</div>
          <div className="wakeup-stat-value">{historyCount}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">连续失败</div>
          <div className="wakeup-stat-value">{failingTasksCount}</div>
        </div>
      </div>

      <div className="card wakeup-global-card">
        <label className="wakeup-toggle">
          <input
            type="checkbox"
            checked={state.enabled}
            onChange={(event) =>
              onStateChange({ ...state, enabled: event.target.checked })
            }
          />
          <span>启用 Wakeup 总开关</span>
        </label>
        <div className="wakeup-hint">
          当前已接入基础 cron 调度器。命中任务时会自动写入历史记录，并支持任务级失败重试、连续失败计数和自动暂停。
        </div>
      </div>
    </>
  );
}
