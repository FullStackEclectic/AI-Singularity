import type { IdeAccount } from "../../types";
import "./SessionsDialogs.css";
import type {
  CodexInstanceCardRecord,
  CodexInstanceRecord,
  CodexSettingsDialogState,
  ConfirmDialogState,
  ProviderOption,
} from "./sessionTypes";

type CodexInstanceCardProps = {
  instance: CodexInstanceRecord;
  sessionCount: number;
  sharedSyncBusyInstanceId: string | null;
  isDefault?: boolean;
  getCodexAccountLabel: (accountId?: string | null) => string;
  getEffectiveCodexAccountId: (instance: CodexInstanceRecord) => string | null | undefined;
  onCopyPath: (instance: CodexInstanceRecord) => void;
  onOpenSettings: (instance: CodexInstanceRecord) => void;
  onCreateFloatingCard: (instance: CodexInstanceRecord) => void;
  onSyncSharedResources: (instance: CodexInstanceRecord) => void;
  onCopyConflictPaths: (instance: CodexInstanceRecord) => void;
  onOpenWindow: (id: string) => void;
  onStop: (id: string) => void;
  onStart: (id: string) => void;
  onDelete?: (id: string) => void;
};

function CodexInstanceCard({
  instance,
  sessionCount,
  sharedSyncBusyInstanceId,
  isDefault = false,
  getCodexAccountLabel,
  getEffectiveCodexAccountId,
  onCopyPath,
  onOpenSettings,
  onCreateFloatingCard,
  onSyncSharedResources,
  onCopyConflictPaths,
  onOpenWindow,
  onStop,
  onStart,
  onDelete,
}: CodexInstanceCardProps) {
  const effectiveAccountId = getEffectiveCodexAccountId(instance);
  const isBusy = sharedSyncBusyInstanceId === instance.id;

  return (
    <div className="session-instance-item" style={isDefault ? { marginBottom: 16 } : undefined}>
      <div>
        <div className="session-instance-title">{instance.name}</div>
        <div className="session-instance-path">{instance.user_data_dir}</div>
        <div className="session-instance-meta">
          <span>{instance.running ? `运行中 PID ${instance.last_pid}` : "未运行"}</span>
          <span>{sessionCount} 条会话</span>
          <span>
            {instance.follow_local_account && isDefault
              ? `跟随当前本地账号 ${getCodexAccountLabel(effectiveAccountId)}`
              : `绑定账号 ${getCodexAccountLabel(instance.bind_account_id)}`}
          </span>
          <span>
            {instance.bind_provider_id
              ? `绑定 Provider ${instance.bind_provider_id}`
              : "使用当前激活 Provider"}
          </span>
          <span>{instance.extra_args ? `参数 ${instance.extra_args}` : "无额外参数"}</span>
        </div>
        <div className="session-instance-flags">
          <span className={`instance-flag ${instance.has_state_db ? "ok" : "bad"}`}>
            state_5.sqlite
          </span>
          <span className={`instance-flag ${instance.has_session_index ? "ok" : "bad"}`}>
            session_index.jsonl
          </span>
          <span className={`instance-flag ${instance.has_shared_skills ? "ok" : "bad"}`}>
            skills
          </span>
          <span className={`instance-flag ${instance.has_shared_rules ? "ok" : "bad"}`}>
            rules
          </span>
          <span
            className={`instance-flag ${
              instance.has_shared_vendor_imports_skills ? "ok" : "bad"
            }`}
          >
            vendor_imports/skills
          </span>
          <span className={`instance-flag ${instance.has_shared_agents_file ? "ok" : "bad"}`}>
            AGENTS.md
          </span>
          {instance.shared_strategy_version && (
            <span className="instance-flag ok">
              共享策略 {instance.shared_strategy_version}
            </span>
          )}
          {instance.has_shared_conflicts && (
            <span
              className="instance-flag bad"
              title={
                (instance.shared_conflict_paths || []).join(", ") ||
                "共享资源存在未托管冲突"
              }
            >
              共享冲突 {(instance.shared_conflict_paths || []).length}
            </span>
          )}
          <span className={`instance-flag ${effectiveAccountId ? "ok" : "bad"}`}>
            {effectiveAccountId
              ? `实际账号 ${getCodexAccountLabel(effectiveAccountId)}`
              : "当前无有效账号"}
          </span>
        </div>
      </div>
      <div className="session-instance-actions">
        {isDefault ? <span className="badge badge-success">默认</span> : null}
        <button className="btn btn-ghost btn-xs" onClick={() => onCopyPath(instance)}>
          复制路径
        </button>
        <button className="btn btn-ghost btn-xs" onClick={() => onOpenSettings(instance)}>
          设置
        </button>
        <button className="btn btn-ghost btn-xs" onClick={() => onCreateFloatingCard(instance)}>
          创建浮窗
        </button>
        <button
          className="btn btn-ghost btn-xs"
          onClick={() => onSyncSharedResources(instance)}
          disabled={isBusy}
        >
          {isBusy ? "同步中..." : "重试共享同步"}
        </button>
        {instance.has_shared_conflicts ? (
          <button
            className="btn btn-ghost btn-xs"
            onClick={() => onCopyConflictPaths(instance)}
            disabled={(instance.shared_conflict_paths || []).length === 0}
          >
            复制冲突清单
          </button>
        ) : null}
        {instance.running ? (
          <>
            <button className="btn btn-ghost btn-xs" onClick={() => onOpenWindow(instance.id)}>
              打开
            </button>
            <button className="btn btn-danger-ghost btn-xs" onClick={() => onStop(instance.id)}>
              停止
            </button>
          </>
        ) : (
          <button
            className="btn btn-primary btn-xs"
            onClick={() => onStart(instance.id)}
            disabled={isBusy}
          >
            {isBusy ? "同步并启动中..." : "启动"}
          </button>
        )}
        {onDelete ? (
          <button
            className="btn btn-danger-ghost btn-xs"
            onClick={() => onDelete(instance.id)}
          >
            删除
          </button>
        ) : null}
      </div>
    </div>
  );
}

type CodexInstancesModalProps = {
  open: boolean;
  codexInstanceCards: CodexInstanceCardRecord[];
  codexInstanceName: string;
  codexInstanceDir: string;
  codexInstanceLoading: boolean;
  sharedSyncBusyInstanceId: string | null;
  onClose: () => void;
  onInstanceNameChange: (value: string) => void;
  onInstanceDirChange: (value: string) => void;
  onPickCodexDir: () => void;
  onAddCodexInstance: () => void;
  onCloseAllInstances: () => void;
  getCodexAccountLabel: (accountId?: string | null) => string;
  getEffectiveCodexAccountId: (instance: CodexInstanceRecord) => string | null | undefined;
  onCopyPath: (instance: CodexInstanceRecord) => void;
  onOpenSettings: (instance: CodexInstanceRecord) => void;
  onCreateFloatingCard: (instance: CodexInstanceRecord) => void;
  onSyncSharedResources: (instance: CodexInstanceRecord) => void;
  onCopyConflictPaths: (instance: CodexInstanceRecord) => void;
  onOpenWindow: (id: string) => void;
  onStop: (id: string) => void;
  onStart: (id: string) => void;
  onDelete: (id: string) => void;
};

export function CodexInstancesModal({
  open,
  codexInstanceCards,
  codexInstanceName,
  codexInstanceDir,
  codexInstanceLoading,
  sharedSyncBusyInstanceId,
  onClose,
  onInstanceNameChange,
  onInstanceDirChange,
  onPickCodexDir,
  onAddCodexInstance,
  onCloseAllInstances,
  getCodexAccountLabel,
  getEffectiveCodexAccountId,
  onCopyPath,
  onOpenSettings,
  onCreateFloatingCard,
  onSyncSharedResources,
  onCopyConflictPaths,
  onOpenWindow,
  onStop,
  onStart,
  onDelete,
}: CodexInstancesModalProps) {
  if (!open) {
    return null;
  }

  const defaultInstance = codexInstanceCards.find((item) => item.is_default);
  const extraInstances = codexInstanceCards.filter((item) => !item.is_default);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Codex 实例目录</h2>
          <button className="btn btn-icon" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">
          <div className="alert alert-info" style={{ marginBottom: 16 }}>
            默认实例 <code>~/.codex</code> 会自动生效，这里只管理额外实例目录。
          </div>

          {defaultInstance ? (
            <CodexInstanceCard
              instance={defaultInstance}
              sessionCount={defaultInstance.sessionCount}
              sharedSyncBusyInstanceId={sharedSyncBusyInstanceId}
              isDefault
              getCodexAccountLabel={getCodexAccountLabel}
              getEffectiveCodexAccountId={getEffectiveCodexAccountId}
              onCopyPath={onCopyPath}
              onOpenSettings={onOpenSettings}
              onCreateFloatingCard={onCreateFloatingCard}
              onSyncSharedResources={onSyncSharedResources}
              onCopyConflictPaths={onCopyConflictPaths}
              onOpenWindow={onOpenWindow}
              onStop={onStop}
              onStart={onStart}
            />
          ) : null}

          <div className="form-row">
            <label className="form-label">实例名称</label>
            <input
              className="form-input"
              value={codexInstanceName}
              onChange={(e) => onInstanceNameChange(e.target.value)}
              placeholder="例如：工作目录实例 / 沙盒实例"
            />
          </div>

          <div className="form-row">
            <label className="form-label">实例目录</label>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                className="form-input font-mono"
                value={codexInstanceDir}
                onChange={(e) => onInstanceDirChange(e.target.value)}
                placeholder="选择或粘贴 Codex user data 目录"
              />
              <button type="button" className="btn btn-ghost" onClick={onPickCodexDir}>
                浏览
              </button>
            </div>
          </div>

          <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 16 }}>
            <button
              className="btn btn-danger-ghost"
              style={{ marginRight: 8 }}
              onClick={onCloseAllInstances}
            >
              全部关闭
            </button>
            <button
              className="btn btn-primary"
              onClick={onAddCodexInstance}
              disabled={codexInstanceLoading}
            >
              {codexInstanceLoading ? "添加中..." : "添加实例"}
            </button>
          </div>

          <div className="session-instance-list">
            {extraInstances.length === 0 ? (
              <div className="empty-text">当前还没有额外 Codex 实例</div>
            ) : (
              extraInstances.map((item) => (
                <CodexInstanceCard
                  key={item.id}
                  instance={item}
                  sessionCount={item.sessionCount}
                  sharedSyncBusyInstanceId={sharedSyncBusyInstanceId}
                  getCodexAccountLabel={getCodexAccountLabel}
                  getEffectiveCodexAccountId={getEffectiveCodexAccountId}
                  onCopyPath={onCopyPath}
                  onOpenSettings={onOpenSettings}
                  onCreateFloatingCard={onCreateFloatingCard}
                  onSyncSharedResources={onSyncSharedResources}
                  onCopyConflictPaths={onCopyConflictPaths}
                  onOpenWindow={onOpenWindow}
                  onStop={onStop}
                  onStart={onStart}
                  onDelete={onDelete}
                />
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

type SessionsConfirmDialogProps = {
  dialog: ConfirmDialogState | null;
  busy: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
};

export function SessionsConfirmDialog({
  dialog,
  busy,
  onClose,
  onConfirm,
}: SessionsConfirmDialogProps) {
  if (!dialog) {
    return null;
  }

  return (
    <div className="modal-overlay" onClick={() => !busy && onClose()}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{dialog.title}</h2>
          <button className="btn btn-icon" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">
          <p>{dialog.description}</p>
          <div className="modal-footer">
            <button className="btn btn-ghost" onClick={onClose} disabled={busy}>
              取消
            </button>
            <button
              className={dialog.tone === "danger" ? "btn btn-danger" : "btn btn-primary"}
              disabled={busy}
              onClick={onConfirm}
            >
              {busy ? "处理中..." : dialog.confirmLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

type CodexInstanceSettingsDialogProps = {
  dialog: CodexSettingsDialogState | null;
  codexAccounts: IdeAccount[];
  codexProviders: ProviderOption[];
  currentCodexAccountId: string | null;
  onClose: () => void;
  onChange: (next: CodexSettingsDialogState) => void;
  onSave: () => Promise<void>;
  getCodexAccountLabel: (accountId?: string | null) => string;
};

export function CodexInstanceSettingsDialog({
  dialog,
  codexAccounts,
  codexProviders,
  currentCodexAccountId,
  onClose,
  onChange,
  onSave,
  getCodexAccountLabel,
}: CodexInstanceSettingsDialogProps) {
  if (!dialog) {
    return null;
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>设置 Codex 实例</h2>
          <button className="btn btn-icon" onClick={onClose}>
            ✕
          </button>
        </div>
        <div
          className="modal-body"
          style={{ display: "flex", flexDirection: "column", gap: 12 }}
        >
          <div className="form-row">
            <label className="form-label">额外启动参数</label>
            <input
              className="form-input"
              value={dialog.extraArgs}
              onChange={(e) => onChange({ ...dialog, extraArgs: e.target.value })}
            />
          </div>
          <div className="form-row">
            <label className="form-label">绑定账号</label>
            <select
              className="form-input"
              value={dialog.bindAccountId}
              onChange={(e) => onChange({ ...dialog, bindAccountId: e.target.value })}
              disabled={dialog.instance.is_default && dialog.followLocalAccount}
            >
              <option value="">未绑定账号</option>
              {codexAccounts.map((account) => (
                <option key={account.id} value={account.id}>
                  {account.label?.trim() || account.email}
                  {currentCodexAccountId === account.id ? " (当前本地)" : ""}
                </option>
              ))}
            </select>
            <div className="text-muted" style={{ fontSize: 12 }}>
              {dialog.instance.is_default && dialog.followLocalAccount
                ? `当前会跟随本地 Codex 账号：${getCodexAccountLabel(currentCodexAccountId)}`
                : `当前选择：${getCodexAccountLabel(dialog.bindAccountId || null)}`}
            </div>
          </div>
          <div className="form-row">
            <label className="form-label">绑定 Provider</label>
            <select
              className="form-input"
              value={dialog.bindProviderId}
              onChange={(e) => onChange({ ...dialog, bindProviderId: e.target.value })}
            >
              <option value="">使用当前激活 Provider</option>
              {codexProviders.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                  {provider.is_active ? " (当前激活)" : ""}
                </option>
              ))}
            </select>
          </div>
          {dialog.instance.is_default ? (
            <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13 }}>
              <input
                type="checkbox"
                checked={dialog.followLocalAccount}
                onChange={(e) =>
                  onChange({ ...dialog, followLocalAccount: e.target.checked })
                }
              />
              跟随当前本地 Codex 账号
            </label>
          ) : null}
          {dialog.instance.is_default &&
          dialog.followLocalAccount &&
          !currentCodexAccountId ? (
            <div className="alert alert-warning" style={{ fontSize: 13 }}>
              当前没有解析到本地 Codex 账号。若继续保持跟随模式，默认实例启动时不会有可注入的账号。
            </div>
          ) : null}
          {(!dialog.instance.is_default || !dialog.followLocalAccount) &&
          dialog.bindAccountId &&
          currentCodexAccountId &&
          dialog.bindAccountId !== currentCodexAccountId ? (
            <div className="alert alert-info" style={{ fontSize: 13 }}>
              这个实例启动时会把当前本地账号从{" "}
              {getCodexAccountLabel(currentCodexAccountId)} 切换为{" "}
              {getCodexAccountLabel(dialog.bindAccountId)}。
            </div>
          ) : null}
          <div className="modal-footer">
            <button className="btn btn-ghost" onClick={onClose}>
              取消
            </button>
            <button className="btn btn-primary" onClick={onSave}>
              保存
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
