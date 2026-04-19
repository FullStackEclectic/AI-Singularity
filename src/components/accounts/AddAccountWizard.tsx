import { useState } from "react";
import { AddAccountApiKeyTab } from "./AddAccountApiKeyTab";
import { AddAccountWizardSandboxPanel } from "./AddAccountWizardSandboxPanel";
import { AddAccountWizardShell } from "./AddAccountWizardShell";
import {
  IDE_ORIGINS,
  LOCAL_IMPORT_OPTIONS,
  isImportOnly,
} from "./addAccountWizardConfig";
import { useAddAccountWizardApiKey } from "./useAddAccountWizardApiKey";
import { useAddAccountWizardSandbox } from "./useAddAccountWizardSandbox";
import type {
  AccountMode,
  ChannelOption,
  IdeOrigin,
  ImportSummary,
  SandboxTab,
  Status,
} from "./addAccountWizardTypes";
import "./AddAccountWizard.css";
import "./AddAccountWizardControls.css";

interface Props {
  onClose: () => void;
  onSuccess: () => void;
}

export default function AddAccountWizard({ onClose, onSuccess }: Props) {
  const [mode, setMode] = useState<AccountMode>("sandbox");
  const [sandboxTab, setSandboxTab] = useState<SandboxTab>("oauth");
  const [ideOrigin, setIdeOrigin] = useState<IdeOrigin>("antigravity");
  const [status, setStatus] = useState<Status>("idle");
  const [message, setMessage] = useState("");
  const [importSummary, setImportSummary] = useState<ImportSummary | null>(null);
  const resetStatus = () => {
    setStatus("idle");
    setMessage("");
    setImportSummary(null);
  };

  const presentImportSummary = (
    summary: ImportSummary,
    successPrefix: string,
    emptyMessage: string,
  ) => {
    setImportSummary(summary);
    if (summary.ok > 0) {
      setStatus("success");
      setMessage(`${successPrefix}：成功 ${summary.ok} 个${summary.fail > 0 ? `，失败 ${summary.fail} 个` : ""}。`);
      if (summary.fail === 0) setTimeout(() => onSuccess(), 1200);
    } else {
      setStatus("error");
      setMessage(emptyMessage);
    }
  };

  const {
    platform,
    keyName,
    secret,
    baseUrl,
    notes,
    setPlatform,
    setKeyName,
    setSecret,
    setBaseUrl,
    setNotes,
    handleSaveApiKey,
  } = useAddAccountWizardApiKey({
    onSuccess,
    setStatus,
    setMessage,
  });

  const {
    deviceFlow,
    oauthUserCodeCopied,
    oauthUrlCopied,
    oauthPolling,
    oauthPreparing,
    oauthTimedOut,
    tokenInput,
    importing,
    setTokenInput,
    handleStartDeviceFlow,
    handleCopyUserCode,
    handleCopyOAuthUrl,
    handleOpenOAuthUrl,
    handleTokenSubmit,
    handlePickImportFiles,
    handleScanLocal,
    handlePickVscdb,
    handleImportV1,
    handleImportLocal,
  } = useAddAccountWizardSandbox({
    mode,
    sandboxTab,
    ideOrigin,
    onSuccess,
    setStatus,
    setMessage,
    presentImportSummary,
  });

  const handleTabChange = (tab: SandboxTab) => {
    setSandboxTab(tab);
    resetStatus();
    setTokenInput("");
    if (tab === "oauth" && isImportOnly(ideOrigin)) {
      setIdeOrigin("antigravity");
    }
  };

  const handleModeChange = (nextMode: AccountMode) => {
    setMode(nextMode);
    resetStatus();
    setSandboxTab("oauth");
  };

  const currentLocalImportOption = LOCAL_IMPORT_OPTIONS[ideOrigin];
  const channelOptions: readonly ChannelOption[] =
    sandboxTab === "oauth"
      ? IDE_ORIGINS.filter((option) => !isImportOnly(option.value))
      : IDE_ORIGINS;

  return (
    <AddAccountWizardShell mode={mode} onClose={onClose} onModeChange={handleModeChange}>
        {mode === "sandbox" && (
          <AddAccountWizardSandboxPanel
            sandboxTab={sandboxTab}
            ideOrigin={ideOrigin}
            status={status}
            message={message}
            importSummary={importSummary}
            channelOptions={channelOptions}
            currentLocalImportOption={currentLocalImportOption ?? null}
            deviceFlow={deviceFlow}
            oauthUserCodeCopied={oauthUserCodeCopied}
            oauthUrlCopied={oauthUrlCopied}
            oauthPolling={oauthPolling}
            oauthPreparing={oauthPreparing}
            oauthTimedOut={oauthTimedOut}
            tokenInput={tokenInput}
            importing={importing}
            onIdeOriginChange={setIdeOrigin}
            onTabChange={handleTabChange}
            onTokenInputChange={setTokenInput}
            onStartDeviceFlow={() => void handleStartDeviceFlow()}
            onCopyUserCode={() => void handleCopyUserCode()}
            onCopyOAuthUrl={() => void handleCopyOAuthUrl()}
            onOpenOAuthUrl={handleOpenOAuthUrl}
            onTokenSubmit={() => void handleTokenSubmit()}
            onScanLocal={() => void handleScanLocal()}
            onPickVscdb={() => void handlePickVscdb()}
            onImportLocal={() =>
              currentLocalImportOption
                ? void handleImportLocal(currentLocalImportOption)
                : undefined
            }
            onPickImportFiles={() => void handlePickImportFiles()}
            onImportV1={() => void handleImportV1()}
          />
        )}

        {mode === "api_key" && (
          <AddAccountApiKeyTab
            status={status}
            message={message}
            importSummary={importSummary}
            platform={platform}
            keyName={keyName}
            secret={secret}
            baseUrl={baseUrl}
            notes={notes}
            onPlatformChange={setPlatform}
            onKeyNameChange={setKeyName}
            onSecretChange={setSecret}
            onBaseUrlChange={setBaseUrl}
            onNotesChange={setNotes}
            onSave={() => void handleSaveApiKey()}
          />
        )}
    </AddAccountWizardShell>
  );
}
