import { Key, Loader2, ShieldCheck } from "lucide-react";
import { PLATFORM_LABELS, type Platform } from "../../types";
import { AddAccountImportSummary } from "./AddAccountImportSummary";
import { AddAccountStatusAlert } from "./AddAccountStatusAlert";
import type { ImportSummary, Status } from "./addAccountWizardTypes";

type AddAccountApiKeyTabProps = {
  status: Status;
  message: string;
  importSummary: ImportSummary | null;
  platform: Platform;
  keyName: string;
  secret: string;
  baseUrl: string;
  notes: string;
  onPlatformChange: (value: Platform) => void;
  onKeyNameChange: (value: string) => void;
  onSecretChange: (value: string) => void;
  onBaseUrlChange: (value: string) => void;
  onNotesChange: (value: string) => void;
  onSave: () => void;
};

export function AddAccountApiKeyTab({
  status,
  message,
  importSummary,
  platform,
  keyName,
  secret,
  baseUrl,
  notes,
  onPlatformChange,
  onKeyNameChange,
  onSecretChange,
  onBaseUrlChange,
  onNotesChange,
  onSave,
}: AddAccountApiKeyTabProps) {
  return (
    <div className="wiz-tab-content">
      <AddAccountStatusAlert status={status} message={message} />
      <AddAccountImportSummary importSummary={importSummary} />

      <div className="wiz-field-row">
        <label className="wiz-field-label">Provider</label>
        <select
          className="wiz-select"
          value={platform}
          onChange={(e) => onPlatformChange(e.target.value as Platform)}
        >
          {Object.entries(PLATFORM_LABELS).map(([k, v]) => (
            <option key={k} value={k}>
              {v as string}
            </option>
          ))}
        </select>
      </div>

      <div className="wiz-field-row">
        <label className="wiz-field-label">标识名称</label>
        <input
          className="wiz-input"
          placeholder="例如：主账户-GPT4o"
          value={keyName}
          onChange={(e) => onKeyNameChange(e.target.value)}
        />
      </div>

      <div className="wiz-field-row">
        <label className="wiz-field-label">API Key</label>
        <input
          className="wiz-input"
          type="password"
          placeholder="sk-..."
          value={secret}
          onChange={(e) => onSecretChange(e.target.value)}
        />
      </div>

      {platform === "custom" && (
        <div className="wiz-field-row">
          <label className="wiz-field-label">Base URL</label>
          <input
            className="wiz-input"
            placeholder="https://api.example.com/v1"
            value={baseUrl}
            onChange={(e) => onBaseUrlChange(e.target.value)}
          />
        </div>
      )}

      <div className="wiz-field-row">
        <label className="wiz-field-label">备注</label>
        <input
          className="wiz-input"
          placeholder="可选"
          value={notes}
          onChange={(e) => onNotesChange(e.target.value)}
        />
      </div>

      <div className="wiz-field-hint">
        <ShieldCheck size={13} /> Key 仅存储于本地，不上传任何服务器
      </div>

      <button
        className="wiz-btn-primary"
        onClick={onSave}
        disabled={status === "loading" || status === "success"}
      >
        {status === "loading" ? <Loader2 size={16} className="spin" /> : <Key size={16} />}
        保存 API Key
      </button>
    </div>
  );
}
