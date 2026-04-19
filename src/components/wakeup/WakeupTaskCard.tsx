import { Play, Trash2 } from "lucide-react";
import type { WakeupTask } from "../../lib/api";
import type {
  AccountGroup,
  IdeAccount,
} from "../../types";
import {
  normalizeClientVersionMode,
  resolveWakeupClientProfile,
  suggestWakeupCommandTemplate,
  WAKEUP_CLIENT_VERSION_OPTIONS,
  WAKEUP_UNGROUPED_FILTER,
} from "./wakeupUtils";

type WakeupTaskCardProps = {
  task: WakeupTask;
  accounts: IdeAccount[];
  accountGroupByAccountId: Map<string, AccountGroup>;
  verificationGroupOptions: AccountGroup[];
  taskGroupFilter: string;
  runningTaskId: string | null;
  getAccountGroupLabel: (accountId?: string) => string;
  getWakeupCategoryLabel: (category?: string | null) => string;
  onTaskGroupFilterChange: (taskId: string, value: string) => void;
  onTaskChange: (id: string, patch: Partial<WakeupTask>) => void;
  onRunTaskNow: (taskId: string) => void;
  onCopyTaskCommand: (task: WakeupTask) => void;
  onDeleteTask: (taskId: string) => void;
};

export function WakeupTaskCard({
  task,
  accounts,
  accountGroupByAccountId,
  verificationGroupOptions,
  taskGroupFilter,
  runningTaskId,
  getAccountGroupLabel,
  getWakeupCategoryLabel,
  onTaskGroupFilterChange,
  onTaskChange,
  onRunTaskNow,
  onCopyTaskCommand,
  onDeleteTask,
}: WakeupTaskCardProps) {
  const taskVisibleAccounts =
    taskGroupFilter === "all"
      ? accounts
      : taskGroupFilter === WAKEUP_UNGROUPED_FILTER
        ? accounts.filter((account) => !accountGroupByAccountId.get(account.id))
        : accounts.filter((account) => accountGroupByAccountId.get(account.id)?.id === taskGroupFilter);

  const selectedAccount = accounts.find((account) => account.id === task.account_id);
  const taskClientProfile = resolveWakeupClientProfile(
    selectedAccount?.origin_platform,
    task.client_version_mode,
    task.client_version_fallback_mode
  );

  return (
    <div className="wakeup-task-item">
      <div className="wakeup-task-grid">
        <label>
          <span>任务名</span>
          <input
            className="form-input"
            value={task.name}
            onChange={(event) => onTaskChange(task.id, { name: event.target.value })}
          />
        </label>
        <label>
          <span>账号分组</span>
          <select
            className="form-input"
            value={taskGroupFilter}
            onChange={(event) => onTaskGroupFilterChange(task.id, event.target.value)}
          >
            <option value="all">全部账号</option>
            <option value={WAKEUP_UNGROUPED_FILTER}>未分组</option>
            {verificationGroupOptions.map((group) => (
              <option key={group.id} value={group.id}>
                {group.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>账号</span>
          <select
            className="form-input"
            value={task.account_id}
            onChange={(event) => {
              const accountId = event.target.value;
              const account = accounts.find((item) => item.id === accountId);
              onTaskChange(task.id, {
                account_id: accountId,
                command_template: task.command_template.trim()
                  ? task.command_template
                  : suggestWakeupCommandTemplate(account?.origin_platform || ""),
              });
            }}
          >
            <option value="">选择 IDE 账号</option>
            {taskVisibleAccounts.map((account) => (
              <option key={account.id} value={account.id}>
                {(account.label || account.email) + " · " + account.origin_platform + " · " + getAccountGroupLabel(account.id)}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>触发模式</span>
          <select
            className="form-input"
            value={task.trigger_mode || "cron"}
            onChange={(event) => onTaskChange(task.id, { trigger_mode: event.target.value })}
          >
            <option value="cron">Cron</option>
            <option value="quota_reset">Quota Reset</option>
          </select>
        </label>
        {(task.trigger_mode || "cron") === "quota_reset" ? (
          <>
            <label>
              <span>重置窗口</span>
              <select
                className="form-input"
                value={task.reset_window || "primary_window"}
                onChange={(event) => onTaskChange(task.id, { reset_window: event.target.value })}
              >
                <option value="primary_window">primary_window</option>
                <option value="secondary_window">secondary_window</option>
                <option value="either_window">either_window</option>
              </select>
            </label>
            <label>
              <span>窗口日期策略</span>
              <select
                className="form-input"
                value={task.window_day_policy || "all_days"}
                onChange={(event) => onTaskChange(task.id, { window_day_policy: event.target.value })}
              >
                <option value="all_days">任意日</option>
                <option value="workdays">仅工作日</option>
                <option value="weekends">仅周末</option>
              </select>
            </label>
            <label>
              <span>窗口回退策略</span>
              <select
                className="form-input"
                value={task.window_fallback_policy || "none"}
                onChange={(event) => onTaskChange(task.id, { window_fallback_policy: event.target.value })}
              >
                <option value="none">不回退</option>
                <option value="primary_then_secondary_on_failure">主窗口失败后回退次窗口</option>
              </select>
            </label>
          </>
        ) : null}
        <label>
          <span>客户端版本模式</span>
          <select
            className="form-input"
            value={normalizeClientVersionMode(task.client_version_mode)}
            onChange={(event) => onTaskChange(task.id, { client_version_mode: event.target.value })}
          >
            {WAKEUP_CLIENT_VERSION_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>模式回退策略</span>
          <select
            className="form-input"
            value={normalizeClientVersionMode(task.client_version_fallback_mode)}
            onChange={(event) =>
              onTaskChange(task.id, { client_version_fallback_mode: event.target.value })
            }
          >
            <option value="auto">auto（推荐）</option>
            <option value="official_stable">official_stable</option>
            <option value="official_preview">official_preview</option>
            <option value="official_legacy">official_legacy</option>
          </select>
        </label>
        <label>
          <span>模型</span>
          <input
            className="form-input"
            value={task.model}
            onChange={(event) => onTaskChange(task.id, { model: event.target.value })}
            placeholder="例如 gpt-4.1 / gemini-2.5-pro"
          />
        </label>
        {(task.trigger_mode || "cron") === "cron" ? (
          <label>
            <span>Cron</span>
            <input
              className="form-input"
              value={task.cron}
              onChange={(event) => onTaskChange(task.id, { cron: event.target.value })}
            />
          </label>
        ) : (
          <label>
            <span>重置来源说明</span>
            <input className="form-input" value="从账号 quota_json 读取 reset_time" disabled />
          </label>
        )}
        <label className="span-2">
          <span>执行命令模板</span>
          <input
            className="form-input"
            value={task.command_template}
            onChange={(event) => onTaskChange(task.id, { command_template: event.target.value })}
            placeholder='例如 gemini -m "{model}" -p "{prompt}"'
          />
        </label>
        <label className="span-2">
          <span>Prompt</span>
          <textarea
            className="form-input wakeup-textarea"
            value={task.prompt}
            onChange={(event) => onTaskChange(task.id, { prompt: event.target.value })}
          />
        </label>
        <label>
          <span>超时秒数</span>
          <input
            className="form-input"
            type="number"
            min={10}
            max={3600}
            value={task.timeout_seconds}
            onChange={(event) =>
              onTaskChange(task.id, {
                timeout_seconds: Math.max(10, Number(event.target.value) || 120),
              })
            }
          />
        </label>
        <label>
          <span>失败重试</span>
          <input
            className="form-input"
            type="number"
            min={0}
            max={5}
            value={task.retry_failed_times ?? 0}
            onChange={(event) =>
              onTaskChange(task.id, {
                retry_failed_times: Math.max(0, Math.min(5, Number(event.target.value) || 0)),
              })
            }
          />
        </label>
        <label>
          <span>失败后自动暂停</span>
          <input
            className="form-input"
            type="number"
            min={0}
            max={20}
            value={task.pause_after_failures ?? 0}
            onChange={(event) =>
              onTaskChange(task.id, {
                pause_after_failures: Math.max(0, Math.min(20, Number(event.target.value) || 0)),
              })
            }
          />
        </label>
        <label className="span-2">
          <span>备注</span>
          <input
            className="form-input"
            value={task.notes || ""}
            onChange={(event) => onTaskChange(task.id, { notes: event.target.value })}
          />
        </label>
      </div>

      <div className="wakeup-task-footer">
        <label className="wakeup-toggle">
          <input
            type="checkbox"
            checked={task.enabled}
            onChange={(event) => onTaskChange(task.id, { enabled: event.target.checked })}
          />
          <span>启用任务</span>
        </label>
        <div className="wakeup-task-template-hint">
          可用占位符：{"{model}"}、{"{prompt}"}、{"{account_id}"}、{"{email}"}、{"{client_runtime_args}"}、{"{gateway_mode}"}、{"{gateway_transport}"}、{"{gateway_routing}"}
        </div>
        <div className="wakeup-task-template-hint">
          当前客户端模式：{taskClientProfile.effectiveMode} · 参数：{taskClientProfile.runtimeArgs || "无"} · 网关：{taskClientProfile.gatewayMode}/{taskClientProfile.gatewayTransport}
          {taskClientProfile.fallbackReason ? ` · ${taskClientProfile.fallbackReason}` : ""}
        </div>
        <div className="wakeup-task-actions">
          <button
            className="btn btn-primary btn-sm"
            onClick={() => onRunTaskNow(task.id)}
            disabled={runningTaskId === task.id}
          >
            <Play size={14} /> {runningTaskId === task.id ? "执行中..." : "立即执行"}
          </button>
          <button className="btn btn-secondary btn-sm" onClick={() => onCopyTaskCommand(task)}>
            <Play size={14} /> 复制渲染命令
          </button>
          <button className="btn btn-danger-ghost btn-sm" onClick={() => onDeleteTask(task.id)}>
            <Trash2 size={14} /> 删除
          </button>
        </div>
      </div>

      {(task.last_run_at || task.last_status || task.last_message) && (
        <div className="wakeup-history-message" style={{ marginTop: 10 }}>
          最近结果：
          {task.last_status ? ` [${task.last_status}]` : ""}
          {task.last_run_at ? ` ${new Date(task.last_run_at).toLocaleString()}` : ""}
          {task.last_message ? ` · ${task.last_message}` : ""}
          {task.last_category ? ` · 分类 ${getWakeupCategoryLabel(task.last_category)}` : ""}
          {(task.consecutive_failures || 0) > 0 ? ` · 连续失败 ${task.consecutive_failures} 次` : ""}
          {task.account_id ? ` · 分组 ${getAccountGroupLabel(task.account_id)}` : ""}
          {` · 客户端模式 ${normalizeClientVersionMode(task.client_version_mode)} -> ${normalizeClientVersionMode(task.client_version_fallback_mode)}`}
          {task.trigger_mode === "quota_reset"
            ? ` · 触发 quota ${task.reset_window || "primary_window"} · ${task.window_day_policy || "all_days"} · ${task.window_fallback_policy || "none"}`
            : ""}
        </div>
      )}
    </div>
  );
}
