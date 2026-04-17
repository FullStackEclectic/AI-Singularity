import type { Dispatch, SetStateAction } from "react";
import type { IdeAccount } from "../../types";

export type ConfirmDialogState = {
  title: string;
  description: string;
  confirmLabel: string;
  tone?: "danger" | "primary";
  action: () => Promise<void> | void;
};

export type GeminiProjectDialogState = {
  account: IdeAccount;
  projects: { project_id: string; project_name?: string | null }[];
  value: string;
};

export type CodexApiKeyDialogState = {
  account: IdeAccount;
  apiKey: string;
  baseUrl: string;
};

export type IdeLabelDialogState = {
  account: IdeAccount;
  label: string;
};

export type BatchIdeTagsDialogState = {
  ids: string[];
  tagsText: string;
  count: number;
  channelLabel: string;
};

type ConfirmDialogModalProps = {
  dialog: ConfirmDialogState | null;
  busy: boolean;
  setBusy: Dispatch<SetStateAction<boolean>>;
  onClose: () => void;
};

export function ConfirmDialogModal({ dialog, busy, setBusy, onClose }: ConfirmDialogModalProps) {
  if (!dialog) return null;

  return (
    <div className="accounts-modal-overlay" onClick={() => !busy && onClose()}>
      <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="accounts-modal-title">{dialog.title}</h3>
        <p className="accounts-modal-desc">{dialog.description}</p>
        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button
            className={dialog.tone === "danger" ? "btn-danger-solid" : "btn-primary"}
            onClick={async () => {
              try {
                setBusy(true);
                await dialog.action();
                onClose();
              } finally {
                setBusy(false);
              }
            }}
            disabled={busy}
          >
            {busy ? "处理中..." : dialog.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

type GeminiProjectDialogModalProps = {
  dialog: GeminiProjectDialogState | null;
  setDialog: Dispatch<SetStateAction<GeminiProjectDialogState | null>>;
  busy: boolean;
  onClose: () => void;
  onClear: () => Promise<void>;
  onSave: () => Promise<void>;
};

export function GeminiProjectDialogModal({
  dialog,
  setDialog,
  busy,
  onClose,
  onClear,
  onSave,
}: GeminiProjectDialogModalProps) {
  if (!dialog) return null;

  return (
    <div className="accounts-modal-overlay" onClick={() => !busy && onClose()}>
      <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="accounts-modal-title">设置 Gemini 项目</h3>
        <p className="accounts-modal-desc">{dialog.account.email}</p>
        <div className="accounts-form-group">
          <label className="accounts-form-label">可选项目</label>
          <select
            className="accounts-form-input"
            value={dialog.value}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, value: e.target.value } : prev))}
            disabled={busy}
          >
            {dialog.projects.map((project) => (
              <option key={project.project_id} value={project.project_id}>
                {project.project_id}
                {project.project_name ? ` (${project.project_name})` : ""}
              </option>
            ))}
          </select>
        </div>
        <div className="accounts-form-group">
          <label className="accounts-form-label">或手动输入 project_id</label>
          <input
            className="accounts-form-input"
            value={dialog.value}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, value: e.target.value } : prev))}
            disabled={busy}
          />
        </div>
        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button
            className="btn-outline"
            disabled={busy || !dialog.account.project_id}
            onClick={onClear}
          >
            清除项目
          </button>
          <button className="btn-primary" disabled={busy} onClick={onSave}>
            {busy ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

type CodexApiKeyDialogModalProps = {
  dialog: CodexApiKeyDialogState | null;
  setDialog: Dispatch<SetStateAction<CodexApiKeyDialogState | null>>;
  busy: boolean;
  onClose: () => void;
  onSave: () => Promise<void>;
};

export function CodexApiKeyDialogModal({
  dialog,
  setDialog,
  busy,
  onClose,
  onSave,
}: CodexApiKeyDialogModalProps) {
  if (!dialog) return null;

  return (
    <div className="accounts-modal-overlay" onClick={() => !busy && onClose()}>
      <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="accounts-modal-title">编辑 Codex API Key</h3>
        <p className="accounts-modal-desc">{dialog.account.email}</p>
        <div className="accounts-form-group">
          <label className="accounts-form-label">API Key</label>
          <input
            className="accounts-form-input"
            value={dialog.apiKey}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, apiKey: e.target.value } : prev))}
            disabled={busy}
          />
        </div>
        <div className="accounts-form-group">
          <label className="accounts-form-label">Base URL</label>
          <input
            className="accounts-form-input"
            placeholder="留空表示清空自定义 Base URL"
            value={dialog.baseUrl}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, baseUrl: e.target.value } : prev))}
            disabled={busy}
          />
        </div>
        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button className="btn-primary" disabled={busy || !dialog.apiKey.trim()} onClick={onSave}>
            {busy ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

type IdeLabelDialogModalProps = {
  dialog: IdeLabelDialogState | null;
  setDialog: Dispatch<SetStateAction<IdeLabelDialogState | null>>;
  busy: boolean;
  onClose: () => void;
  onSave: () => Promise<void>;
};

export function IdeLabelDialogModal({
  dialog,
  setDialog,
  busy,
  onClose,
  onSave,
}: IdeLabelDialogModalProps) {
  if (!dialog) return null;

  return (
    <div className="accounts-modal-overlay" onClick={() => !busy && onClose()}>
      <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="accounts-modal-title">编辑账号备注名</h3>
        <p className="accounts-modal-desc">{dialog.account.email}</p>
        <div className="accounts-form-group">
          <label className="accounts-form-label">备注名</label>
          <input
            className="accounts-form-input"
            placeholder="留空则恢复显示邮箱"
            value={dialog.label}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, label: e.target.value } : prev))}
            disabled={busy}
          />
        </div>
        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button className="btn-primary" disabled={busy} onClick={onSave}>
            {busy ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

type BatchIdeTagsDialogModalProps = {
  dialog: BatchIdeTagsDialogState | null;
  setDialog: Dispatch<SetStateAction<BatchIdeTagsDialogState | null>>;
  busy: boolean;
  onClose: () => void;
  onSave: () => Promise<void>;
};

export function BatchIdeTagsDialogModal({
  dialog,
  setDialog,
  busy,
  onClose,
  onSave,
}: BatchIdeTagsDialogModalProps) {
  if (!dialog) return null;

  return (
    <div className="accounts-modal-overlay" onClick={() => !busy && onClose()}>
      <div className="accounts-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="accounts-modal-title">批量编辑 IDE 标签</h3>
        <p className="accounts-modal-desc">
          当前将更新 {dialog.channelLabel} 下的 {dialog.count} 个 IDE 账号。
        </p>
        <div className="accounts-form-group">
          <label className="accounts-form-label">标签（用逗号分隔）</label>
          <input
            className="accounts-form-input"
            placeholder="例如：生产, vip, cursor"
            value={dialog.tagsText}
            onChange={(e) => setDialog((prev) => (prev ? { ...prev, tagsText: e.target.value } : prev))}
            disabled={busy}
          />
        </div>
        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button className="btn-primary" disabled={busy} onClick={onSave}>
            {busy ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
