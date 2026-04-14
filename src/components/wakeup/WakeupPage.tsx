import { useEffect, useMemo, useState } from "react";
import { Play, Save, Trash2, History, Clock3 } from "lucide-react";
import {
  api,
  type WakeupHistoryItem,
  type WakeupState,
  type WakeupTask,
  type WakeupVerificationBatchResult,
} from "../../lib/api";
import "./WakeupPage.css";

const createTask = (): WakeupTask => ({
  id: `wakeup-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  name: "",
  enabled: true,
  account_id: "",
  command_template: "",
  model: "",
  prompt: "hi",
  cron: "0 */6 * * *",
  notes: "",
  timeout_seconds: 120,
  created_at: "",
  updated_at: "",
  last_run_at: null,
});

export default function WakeupPage() {
  const [state, setState] = useState<WakeupState>({ enabled: false, tasks: [] });
  const [history, setHistory] = useState<WakeupHistoryItem[]>([]);
  const [accounts, setAccounts] = useState<any[]>([]);
  const [message, setMessage] = useState("");
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [verificationSelection, setVerificationSelection] = useState<string[]>([]);
  const [verificationModel, setVerificationModel] = useState("");
  const [verificationPrompt, setVerificationPrompt] = useState("hi");
  const [verificationCommandTemplate, setVerificationCommandTemplate] = useState('gemini -m "{model}" -p "{prompt}"');
  const [verificationTimeout, setVerificationTimeout] = useState(120);
  const [verificationRunning, setVerificationRunning] = useState(false);
  const [verificationResult, setVerificationResult] = useState<WakeupVerificationBatchResult | null>(null);
  const [runningTaskId, setRunningTaskId] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      setLoading(true);
      try {
        const [wakeupState, wakeupHistory, ideAccounts] = await Promise.all([
          api.wakeup.getState(),
          api.wakeup.loadHistory(),
          api.ideAccounts.list(),
        ]);
        if (!cancelled) {
          setState(wakeupState);
          setHistory(wakeupHistory);
          setAccounts(ideAccounts);
        }
      } catch (e) {
        if (!cancelled) setMessage("加载 Wakeup 数据失败: " + String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const activeTasks = useMemo(
    () => state.tasks.filter((task) => task.enabled),
    [state.tasks]
  );

  const saveState = async (next: WakeupState, successMessage = "Wakeup 状态已保存") => {
    setSaving(true);
    try {
      const saved = await api.wakeup.saveState(next);
      setState(saved);
      setMessage(successMessage);
    } catch (e) {
      setMessage("保存 Wakeup 状态失败: " + String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleTaskChange = (id: string, patch: Partial<WakeupTask>) => {
    setState((prev) => ({
      ...prev,
      tasks: prev.tasks.map((task) => (task.id === id ? { ...task, ...patch } : task)),
    }));
  };

  const suggestCommandTemplate = (platform: string) => {
    const normalized = String(platform || "").toLowerCase();
    if (normalized === "gemini") {
      return 'gemini -m "{model}" -p "{prompt}"';
    }
    if (normalized === "codex") {
      return 'codex "{prompt}"';
    }
    if (normalized === "claude_code" || normalized === "antigravity") {
      return 'claude "{prompt}"';
    }
    return '"{prompt}"';
  };

  const handleAddTask = () => {
    setState((prev) => ({ ...prev, tasks: [createTask(), ...prev.tasks] }));
  };

  const handleDeleteTask = (id: string) => {
    setState((prev) => ({ ...prev, tasks: prev.tasks.filter((task) => task.id !== id) }));
  };

  const handleRunTaskNow = async (taskId: string) => {
    setRunningTaskId(taskId);
    try {
      const nextState = await api.wakeup.runTaskNow(taskId);
      setState(nextState);
      setHistory(await api.wakeup.loadHistory());
      const task = nextState.tasks.find((item) => item.id === taskId);
      setMessage(task?.last_message || "任务已执行");
    } catch (e) {
      setMessage("立即执行任务失败: " + String(e));
    } finally {
      setRunningTaskId(null);
    }
  };

  const handleRecordManualRun = async (task: WakeupTask) => {
    const item: WakeupHistoryItem = {
      id: `history-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      task_id: task.id,
      task_name: task.name || "未命名任务",
      account_id: task.account_id,
      model: task.model,
      status: "manual_marked",
      message: "已记录一次手动执行占位结果。调度器与真实唤醒链将在下一阶段接入。",
      created_at: new Date().toISOString(),
    };
    try {
      const next = await api.wakeup.addHistory([item]);
      setHistory(next);
      setMessage("已记录一条 Wakeup 历史");
    } catch (e) {
      setMessage("记录 Wakeup 历史失败: " + String(e));
    }
  };

  const clearHistory = async () => {
    try {
      await api.wakeup.clearHistory();
      setHistory([]);
      setMessage("Wakeup 历史已清空");
    } catch (e) {
      setMessage("清空 Wakeup 历史失败: " + String(e));
    }
  };

  const toggleVerificationAccount = (accountId: string) => {
    setVerificationSelection((prev) =>
      prev.includes(accountId) ? prev.filter((item) => item !== accountId) : [...prev, accountId]
    );
  };

  const runVerification = async () => {
    if (verificationSelection.length === 0) {
      setMessage("请至少选择一个账号进行批次验证");
      return;
    }
    if (!verificationModel.trim() || !verificationCommandTemplate.trim()) {
      setMessage("请填写验证模型和命令模板");
      return;
    }

    setVerificationRunning(true);
    try {
      const result = await api.wakeup.runVerificationBatch({
        accountIds: verificationSelection,
        model: verificationModel.trim(),
        prompt: verificationPrompt,
        commandTemplate: verificationCommandTemplate.trim(),
        timeoutSeconds: verificationTimeout,
      });
      setVerificationResult(result);
      setHistory(await api.wakeup.loadHistory());
      setMessage(`批次验证完成：成功 ${result.success_count}，失败 ${result.failed_count}`);
    } catch (e) {
      setMessage("批次验证失败: " + String(e));
    } finally {
      setVerificationRunning(false);
    }
  };

  return (
    <div className="wakeup-page">
      <div className="page-header">
        <div>
          <h1 className="page-title"><Clock3 size={22} className="text-primary" /> Wakeup / Verification</h1>
          <p className="page-subtitle">任务、历史和基础 cron 调度已接入。当前调度命中后会记录执行历史并回写 `last_run_at`，真实唤醒调用链和批次验证继续下一批补齐。</p>
        </div>
        <div className="wakeup-header-actions">
          <button className="btn btn-secondary" onClick={handleAddTask}>新增任务</button>
          <button className="btn btn-primary" onClick={() => saveState(state)} disabled={saving}>
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
          <div className="wakeup-stat-value">{activeTasks.length}</div>
        </div>
        <div className="card wakeup-stat">
          <div className="wakeup-stat-label">历史记录</div>
          <div className="wakeup-stat-value">{history.length}</div>
        </div>
      </div>

      <div className="card wakeup-global-card">
        <label className="wakeup-toggle">
          <input
            type="checkbox"
            checked={state.enabled}
            onChange={(e) => setState((prev) => ({ ...prev, enabled: e.target.checked }))}
          />
          <span>启用 Wakeup 总开关</span>
        </label>
        <div className="wakeup-hint">
          当前已接入基础 cron 调度器。命中任务时会自动写入历史记录，并通过全局数据变更事件刷新页面。
        </div>
      </div>

      <div className="wakeup-layout">
        <section className="card wakeup-task-panel">
          <div className="wakeup-section-title">任务列表</div>
          {loading ? (
            <div className="wakeup-empty">加载中...</div>
          ) : state.tasks.length === 0 ? (
            <div className="wakeup-empty">当前还没有 Wakeup 任务</div>
          ) : (
            <div className="wakeup-task-list">
              {state.tasks.map((task) => (
                <div key={task.id} className="wakeup-task-item">
                  <div className="wakeup-task-grid">
                    <label>
                      <span>任务名</span>
                      <input className="form-input" value={task.name} onChange={(e) => handleTaskChange(task.id, { name: e.target.value })} />
                    </label>
                    <label>
                      <span>账号</span>
                      <select
                        className="form-input"
                        value={task.account_id}
                        onChange={(e) => {
                          const accountId = e.target.value;
                          const account = accounts.find((item) => item.id === accountId);
                          handleTaskChange(task.id, {
                            account_id: accountId,
                            command_template: task.command_template.trim()
                              ? task.command_template
                              : suggestCommandTemplate(account?.origin_platform),
                          });
                        }}
                      >
                        <option value="">选择 IDE 账号</option>
                        {accounts.map((account) => (
                          <option key={account.id} value={account.id}>
                            {(account.label || account.email) + " · " + account.origin_platform}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>模型</span>
                      <input className="form-input" value={task.model} onChange={(e) => handleTaskChange(task.id, { model: e.target.value })} placeholder="例如 gpt-4.1 / gemini-2.5-pro" />
                    </label>
                    <label>
                      <span>Cron</span>
                      <input className="form-input" value={task.cron} onChange={(e) => handleTaskChange(task.id, { cron: e.target.value })} />
                    </label>
                    <label className="span-2">
                      <span>执行命令模板</span>
                      <input
                        className="form-input"
                        value={task.command_template}
                        onChange={(e) => handleTaskChange(task.id, { command_template: e.target.value })}
                        placeholder='例如 gemini -m "{model}" -p "{prompt}"'
                      />
                    </label>
                    <label className="span-2">
                      <span>Prompt</span>
                      <textarea className="form-input wakeup-textarea" value={task.prompt} onChange={(e) => handleTaskChange(task.id, { prompt: e.target.value })} />
                    </label>
                    <label>
                      <span>超时秒数</span>
                      <input
                        className="form-input"
                        type="number"
                        min={10}
                        max={3600}
                        value={task.timeout_seconds}
                        onChange={(e) => handleTaskChange(task.id, { timeout_seconds: Math.max(10, Number(e.target.value) || 120) })}
                      />
                    </label>
                    <label className="span-2">
                      <span>备注</span>
                      <input className="form-input" value={task.notes || ""} onChange={(e) => handleTaskChange(task.id, { notes: e.target.value })} />
                    </label>
                  </div>
                  <div className="wakeup-task-footer">
                    <label className="wakeup-toggle">
                      <input
                        type="checkbox"
                        checked={task.enabled}
                        onChange={(e) => handleTaskChange(task.id, { enabled: e.target.checked })}
                      />
                      <span>启用任务</span>
                    </label>
                    <div className="wakeup-task-template-hint">
                      可用占位符：{"{model}"}、{"{prompt}"}、{"{account_id}"}、{"{email}"}
                    </div>
                    <div className="wakeup-task-actions">
                      <button className="btn btn-primary btn-sm" onClick={() => handleRunTaskNow(task.id)} disabled={runningTaskId === task.id}>
                        <Play size={14} /> {runningTaskId === task.id ? "执行中..." : "立即执行"}
                      </button>
                      <button className="btn btn-secondary btn-sm" onClick={() => handleRecordManualRun(task)}>
                        <Play size={14} /> 记录一次手动执行
                      </button>
                      <button className="btn btn-danger-ghost btn-sm" onClick={() => handleDeleteTask(task.id)}>
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
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>

        <div className="wakeup-side-stack">
          <section className="card wakeup-history-panel">
            <div className="wakeup-history-header">
              <div className="wakeup-section-title"><Play size={16} /> 批次验证</div>
              <button className="btn btn-primary btn-sm" onClick={runVerification} disabled={verificationRunning}>
                {verificationRunning ? "执行中..." : "开始验证"}
              </button>
            </div>
            <div className="wakeup-verification-config">
              <label>
                <span>验证模型</span>
                <input className="form-input" value={verificationModel} onChange={(e) => setVerificationModel(e.target.value)} placeholder="例如 gemini-2.5-pro" />
              </label>
              <label>
                <span>Prompt</span>
                <textarea className="form-input wakeup-textarea" value={verificationPrompt} onChange={(e) => setVerificationPrompt(e.target.value)} />
              </label>
              <label>
                <span>命令模板</span>
                <input className="form-input" value={verificationCommandTemplate} onChange={(e) => setVerificationCommandTemplate(e.target.value)} />
              </label>
              <label>
                <span>超时秒数</span>
                <input className="form-input" type="number" min={10} max={3600} value={verificationTimeout} onChange={(e) => setVerificationTimeout(Math.max(10, Number(e.target.value) || 120))} />
              </label>
            </div>
            <div className="wakeup-verification-accounts">
              {accounts.length === 0 ? (
                <div className="wakeup-empty">当前没有可验证的 IDE 账号</div>
              ) : (
                accounts.map((account) => (
                  <label key={account.id} className="wakeup-verification-account">
                    <input
                      type="checkbox"
                      checked={verificationSelection.includes(account.id)}
                      onChange={() => toggleVerificationAccount(account.id)}
                    />
                    <span>{(account.label || account.email) + " · " + account.origin_platform}</span>
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
                </div>
                <div className="wakeup-history-list">
                  {verificationResult.items.map((item) => (
                    <div key={item.account_id} className="wakeup-history-item">
                      <div className="wakeup-history-top">
                        <strong>{item.email}</strong>
                        <span>{item.status}</span>
                      </div>
                      <div className="wakeup-history-message">{item.message}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </section>

          <section className="card wakeup-history-panel">
            <div className="wakeup-history-header">
              <div className="wakeup-section-title"><History size={16} /> 执行历史</div>
              <button className="btn btn-danger-ghost btn-sm" onClick={clearHistory}>清空历史</button>
            </div>
            {history.length === 0 ? (
              <div className="wakeup-empty">当前还没有 Wakeup 历史</div>
            ) : (
              <div className="wakeup-history-list">
                {history.map((item) => (
                  <div key={item.id} className="wakeup-history-item">
                    <div className="wakeup-history-top">
                      <strong>{item.task_name}</strong>
                      <span>{new Date(item.created_at).toLocaleString()}</span>
                    </div>
                    <div className="wakeup-history-meta">
                      <span>账号：{item.account_id || "—"}</span>
                      <span>模型：{item.model || "—"}</span>
                      <span>状态：{item.status}</span>
                    </div>
                    {item.message && <div className="wakeup-history-message">{item.message}</div>}
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
