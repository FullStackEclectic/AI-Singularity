import type { ReactNode } from "react";
import { Database, Globe, Key } from "lucide-react";
import { AddAccountChannelSelect } from "./AddAccountChannelSelect";
import { AddAccountImportSummary } from "./AddAccountImportSummary";
import { AddAccountImportTab } from "./AddAccountImportTab";
import { AddAccountOAuthTab } from "./AddAccountOAuthTab";
import { AddAccountStatusAlert } from "./AddAccountStatusAlert";
import { AddAccountTokenTab } from "./AddAccountTokenTab";
import {
  IDE_ORIGINS,
  isBrowserOAuth,
  isDeviceFlow,
  isImportOnly,
} from "./addAccountWizardConfig";
import type {
  ChannelOption,
  DeviceFlowStart,
  IdeOrigin,
  ImportSummary,
  LocalImportOptionView,
  SandboxTab,
  Status,
} from "./addAccountWizardTypes";

type AddAccountWizardSandboxPanelProps = {
  sandboxTab: SandboxTab;
  ideOrigin: IdeOrigin;
  status: Status;
  message: string;
  importSummary: ImportSummary | null;
  channelOptions: readonly ChannelOption[];
  currentLocalImportOption: LocalImportOptionView | null;
  deviceFlow: DeviceFlowStart | null;
  oauthUserCodeCopied: boolean;
  oauthUrlCopied: boolean;
  oauthPolling: boolean;
  oauthPreparing: boolean;
  oauthTimedOut: boolean;
  tokenInput: string;
  importing: boolean;
  onIdeOriginChange: (value: IdeOrigin) => void;
  onTabChange: (tab: SandboxTab) => void;
  onTokenInputChange: (value: string) => void;
  onStartDeviceFlow: () => void;
  onCopyUserCode: () => void;
  onCopyOAuthUrl: () => void;
  onOpenOAuthUrl: () => void;
  onTokenSubmit: () => void;
  onScanLocal: () => void;
  onPickVscdb: () => void;
  onImportLocal: () => void;
  onPickImportFiles: () => void;
  onImportV1: () => void;
};

const SANDBOX_TABS: {
  tab: SandboxTab;
  label: string;
  icon: ReactNode;
}[] = [
  { tab: "oauth", label: "OAuth 授权", icon: <Globe size={14} /> },
  { tab: "token", label: "Token 粘贴", icon: <Key size={14} /> },
  { tab: "import", label: "导入账号", icon: <Database size={14} /> },
];

export function AddAccountWizardSandboxPanel({
  sandboxTab,
  ideOrigin,
  status,
  message,
  importSummary,
  channelOptions,
  currentLocalImportOption,
  deviceFlow,
  oauthUserCodeCopied,
  oauthUrlCopied,
  oauthPolling,
  oauthPreparing,
  oauthTimedOut,
  tokenInput,
  importing,
  onIdeOriginChange,
  onTabChange,
  onTokenInputChange,
  onStartDeviceFlow,
  onCopyUserCode,
  onCopyOAuthUrl,
  onOpenOAuthUrl,
  onTokenSubmit,
  onScanLocal,
  onPickVscdb,
  onImportLocal,
  onPickImportFiles,
  onImportV1,
}: AddAccountWizardSandboxPanelProps) {
  return (
    <>
      <div className="wiz-channel-row">
        <label className="wiz-channel-label">渠道来源</label>
        <AddAccountChannelSelect
          value={ideOrigin}
          options={channelOptions}
          onChange={(value) => onIdeOriginChange(value as IdeOrigin)}
        />
      </div>

      <div className="wiz-tabs">
        {SANDBOX_TABS.map(({ tab, label, icon }) => (
          <button
            key={tab}
            className={`wiz-tab ${sandboxTab === tab ? "active" : ""}`}
            onClick={() => onTabChange(tab)}
          >
            {icon}
            {label}
          </button>
        ))}
      </div>

      <AddAccountStatusAlert status={status} message={message} />
      <AddAccountImportSummary importSummary={importSummary} />

      {sandboxTab === "oauth" && (
        <AddAccountOAuthTab
          ideOriginLabel={IDE_ORIGINS.find((o) => o.value === ideOrigin)?.label || ideOrigin}
          isImportOnly={isImportOnly(ideOrigin)}
          isBrowserOAuth={isBrowserOAuth(ideOrigin)}
          isDeviceFlow={isDeviceFlow(ideOrigin)}
          status={status}
          deviceFlow={deviceFlow}
          oauthPreparing={oauthPreparing}
          oauthUserCodeCopied={oauthUserCodeCopied}
          oauthUrlCopied={oauthUrlCopied}
          oauthPolling={oauthPolling}
          oauthTimedOut={oauthTimedOut}
          onStartDeviceFlow={onStartDeviceFlow}
          onGoImportTab={() => onTabChange("import")}
          onCopyUserCode={onCopyUserCode}
          onCopyOAuthUrl={onCopyOAuthUrl}
          onOpenOAuthUrl={onOpenOAuthUrl}
        />
      )}

      {sandboxTab === "token" && (
        <AddAccountTokenTab
          ideOrigin={ideOrigin}
          status={status}
          tokenInput={tokenInput}
          onTokenInputChange={onTokenInputChange}
          onSubmit={onTokenSubmit}
        />
      )}

      {sandboxTab === "import" && (
        <AddAccountImportTab
          importing={importing}
          status={status}
          currentLocalImportOption={currentLocalImportOption}
          onScanLocal={onScanLocal}
          onPickVscdb={onPickVscdb}
          onImportLocal={onImportLocal}
          onPickImportFiles={onPickImportFiles}
          onImportV1={onImportV1}
        />
      )}
    </>
  );
}
