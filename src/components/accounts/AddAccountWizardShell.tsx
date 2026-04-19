import type { ReactNode } from "react";
import { Globe, Key, ShieldCheck } from "lucide-react";
import type { AccountMode } from "./addAccountWizardTypes";

type AddAccountWizardShellProps = {
  mode: AccountMode;
  onClose: () => void;
  onModeChange: (mode: AccountMode) => void;
  children: ReactNode;
};

export function AddAccountWizardShell({
  mode,
  onClose,
  onModeChange,
  children,
}: AddAccountWizardShellProps) {
  return (
    <div className="wiz-overlay" onClick={onClose}>
      <div className="wiz-panel" onClick={(e) => e.stopPropagation()}>
        <div className="wiz-header">
          <div className="wiz-header-title">
            <ShieldCheck size={20} />
            <span>添加账号</span>
          </div>
          <button className="wiz-close-btn" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="wiz-mode-row">
          <button
            className={`wiz-mode-btn ${mode === "sandbox" ? "active" : ""}`}
            onClick={() => onModeChange("sandbox")}
          >
            <Globe size={16} />
            <span>沙盒账号（IDE / OAuth）</span>
          </button>
          <button
            className={`wiz-mode-btn ${mode === "api_key" ? "active" : ""}`}
            onClick={() => onModeChange("api_key")}
          >
            <Key size={16} />
            <span>API 密钥（Cloud）</span>
          </button>
        </div>

        {children}
      </div>
    </div>
  );
}
