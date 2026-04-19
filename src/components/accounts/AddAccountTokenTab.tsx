import { Loader2, Upload } from "lucide-react";
import type { Status } from "./addAccountWizardTypes";

type AddAccountTokenTabProps = {
  ideOrigin: string;
  status: Status;
  tokenInput: string;
  onTokenInputChange: (value: string) => void;
  onSubmit: () => void;
};

export function AddAccountTokenTab({
  ideOrigin,
  status,
  tokenInput,
  onTokenInputChange,
  onSubmit,
}: AddAccountTokenTabProps) {
  return (
    <div className="wiz-tab-content">
      <p className="wiz-field-hint">
        支持单个 Token、批量 JSON 数组，或包含多个 <code>1//xxx</code> 的文本。
        {ideOrigin === "codex" ? (
          <>
            {" "}
            也支持直接粘贴 <code>sk-...</code> 作为 Codex API Key 账号导入。
          </>
        ) : null}
      </p>
      <textarea
        className="wiz-textarea"
        placeholder={`粘贴 Token 内容，例如：\n1//xxxxxxxx...\n或 JSON 数组 [{"refresh_token": "1//xxx"}]`}
        value={tokenInput}
        onChange={(e) => onTokenInputChange(e.target.value)}
        rows={7}
        disabled={status === "loading" || status === "success"}
      />
      <button
        className="wiz-btn-primary"
        onClick={onSubmit}
        disabled={status === "loading" || status === "success" || !tokenInput.trim()}
      >
        {status === "loading" ? <Loader2 size={16} className="spin" /> : <Upload size={16} />}
        批量导入
      </button>
    </div>
  );
}
