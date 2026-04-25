import { useCallback, useEffect, useState } from "react";
import { Activity, Clock3, Save } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  type WakeupRuntimeStatus,
  type WakeupState,
  type WakeupSummary24h,
} from "../../lib/api";

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
  const [runtime, setRuntime] = useState<WakeupRuntimeStatus | null>(null);
  const [summary, setSummary] = useState<WakeupSummary24h | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [rt, sm] = await Promise.all([
        api.wakeup.getRuntimeStatus(),
        api.wakeup.getSummary24h(),
      ]);
      setRuntime(rt);
      setSummary(sm);
    } catch {
      /* metrics endpoints failing should not break the page */
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 30_000);
    const unlistenPromise = listen<{ domain?: string }>(
      "data:changed",
      (event) => {
        if (event.payload?.domain === "wakeup") void refresh();
      },
    );
    return () => {
      window.clearInterval(interval);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refresh]);

  const successRate24h =
    summary && summary.totalCount > 0
      ? summary.successCount / summary.totalCount
      : null;
  const successRateLabel =
    successRate24h === null ? "—" : `${(successRate24h * 100).toFixed(0)}%`;

  return (
    <>
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <Clock3 size={22} className="text-primary" /> Wakeup / Verification
          </h1>
          <p className="page-subtitle">
            统一网关派发、Run 聚合、设备健康联动；任务、历史、cron 调度、即时执行和批次验证一站接入。
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
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">
            <Activity size={12} /> 网关并发
          </div>
          <div className="wakeup-stat-value">
            {runtime
              ? `${runtime.concurrencyInUse} / ${runtime.concurrencyLimit}`
              : "—"}
          </div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">24h 成功率</div>
          <div className="wakeup-stat-value">{successRateLabel}</div>
        </div>
      </div>

      {summary && summary.categories.length > 0 && (
        <div className="card wakeup-distribution-card">
          <div className="wakeup-section-title">
            近 24h 触发分布（共 {summary.totalCount} 次：成功 {summary.successCount}，失败 {summary.failureCount}）
          </div>
          <CategoryBars summary={summary} />
        </div>
      )}

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
          已切换到 SQLite 持久化 + WakeupGateway 中枢：自动健康预检、Run 生命周期记录、与 IDE
          账号失效检测形成双向闭环。
        </div>
      </div>
    </>
  );
}

function CategoryBars({ summary }: { summary: WakeupSummary24h }) {
  const max = Math.max(...summary.categories.map((c) => c.total), 1);
  return (
    <div className="wakeup-distribution-bars">
      {summary.categories.map((category) => {
        const widthPct = (category.total / max) * 100;
        const successRatio =
          category.total > 0 ? category.success / category.total : 0;
        const tone =
          successRatio >= 0.8
            ? "success"
            : successRatio >= 0.5
              ? "warn"
              : "danger";
        return (
          <div key={category.category} className="wakeup-distribution-row">
            <div className="wakeup-distribution-label">{category.category}</div>
            <div className="wakeup-distribution-track">
              <div
                className={`wakeup-distribution-fill wakeup-distribution-${tone}`}
                style={{ width: `${widthPct}%` }}
              />
            </div>
            <div className="wakeup-distribution-count">
              {category.success} / {category.total}
            </div>
          </div>
        );
      })}
    </div>
  );
}
