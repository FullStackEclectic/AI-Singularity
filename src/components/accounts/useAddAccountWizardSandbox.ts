import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import { api } from "../../lib/api";
import type {
  AccountMode,
  DeviceFlowStart,
  FileImportScanResult,
  IdeOrigin,
  ImportSummary,
  LocalImportOption,
  SandboxTab,
  ScannedIdeAccount,
  Status,
} from "./addAccountWizardTypes";
import { importScannedAccounts, mergeFileImportSummary } from "./addAccountWizardUtils";

type UseAddAccountWizardSandboxParams = {
  mode: AccountMode;
  sandboxTab: SandboxTab;
  ideOrigin: IdeOrigin;
  onSuccess: () => void;
  setStatus: Dispatch<SetStateAction<Status>>;
  setMessage: Dispatch<SetStateAction<string>>;
  presentImportSummary: (
    summary: ImportSummary,
    successPrefix: string,
    emptyMessage: string
  ) => void;
};

export function useAddAccountWizardSandbox({
  mode,
  sandboxTab,
  ideOrigin,
  onSuccess,
  setStatus,
  setMessage,
  presentImportSummary,
}: UseAddAccountWizardSandboxParams) {
  const [deviceFlow, setDeviceFlow] = useState<DeviceFlowStart | null>(null);
  const [oauthUserCodeCopied, setOauthUserCodeCopied] = useState(false);
  const [oauthUrlCopied, setOauthUrlCopied] = useState(false);
  const [oauthPolling, setOauthPolling] = useState(false);
  const [oauthPreparing, setOauthPreparing] = useState(false);
  const [oauthTimedOut, setOauthTimedOut] = useState(false);
  const [tokenInput, setTokenInput] = useState("");
  const [importing, setImporting] = useState(false);

  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const loginIdRef = useRef<string | null>(null);

  const clearPoll = useCallback(() => {
    if (pollIntervalRef.current) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  const cancelDeviceFlow = useCallback(() => {
    clearPoll();
    const id = loginIdRef.current;
    if (id) {
      invoke("cancel_oauth_flow", { loginId: id }).catch(() => {});
      loginIdRef.current = null;
    }
    setDeviceFlow(null);
    setOauthPolling(false);
    setOauthPreparing(false);
    setOauthTimedOut(false);
  }, [clearPoll]);

  useEffect(() => {
    if (sandboxTab !== "oauth" || mode !== "sandbox") {
      cancelDeviceFlow();
    }
  }, [cancelDeviceFlow, mode, sandboxTab]);

  useEffect(() => () => {
    cancelDeviceFlow();
  }, [cancelDeviceFlow]);

  const startPolling = useCallback(
    (intervalSecs: number) => {
      clearPoll();
      setOauthPolling(true);
      pollIntervalRef.current = setInterval(async () => {
        if (!loginIdRef.current) {
          clearPoll();
          return;
        }
        try {
          const result = await invoke<{
            done: boolean;
            token?: string;
            access_token?: string;
            refresh_token?: string;
            meta_json?: string | null;
            email?: string;
          }>("poll_oauth_login", { loginId: loginIdRef.current });

          if (result.done && result.token) {
            clearPoll();
            setOauthPolling(false);
            const origin = ideOrigin;
            const accessToken =
              result.access_token ||
              (origin === "gemini" || origin === "antigravity"
                ? "requires_refresh"
                : result.token);
            const refreshToken =
              result.refresh_token ||
              (origin === "gemini" || origin === "antigravity"
                ? result.token
                : "missing");
            const accountEmail = result.email || `${origin}-${Date.now()}@oauth.local`;

            await api.ideAccounts.import([
              {
                id: `oauth-${Date.now()}`,
                email: accountEmail,
                origin_platform: origin,
                token: {
                  access_token: accessToken,
                  refresh_token: refreshToken,
                  expires_in: 3600,
                  token_type: "Bearer",
                  updated_at: new Date().toISOString(),
                },
                status: "active",
                is_proxy_disabled: false,
                device_profile: {
                  machine_id: `sys-${Math.random().toString(36).substring(2, 10)}`,
                  mac_machine_id: `mac-${Math.random().toString(36).substring(2, 10)}`,
                  dev_device_id: crypto.randomUUID(),
                  sqm_id: `{${crypto.randomUUID()}}`,
                },
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
                last_used: new Date().toISOString(),
                meta_json: result.meta_json || null,
              },
            ]);

            setStatus("success");
            setMessage(
              `OAuth 授权成功！账号已导入${result.email ? `（${result.email}）` : ""}。`
            );
            loginIdRef.current = null;
            setTimeout(() => onSuccess(), 1200);
          }
        } catch (error) {
          clearPoll();
          setOauthPolling(false);
          const message = String(error);
          if (message.includes("过期")) {
            setOauthTimedOut(true);
            setStatus("error");
            setMessage("授权码已过期，请点击「重新获取」");
          } else if (!message.includes("取消")) {
            setStatus("error");
            setMessage("授权失败: " + message);
          }
        }
      }, intervalSecs * 1000);
    },
    [clearPoll, ideOrigin, onSuccess, setMessage, setStatus]
  );

  const handleStartDeviceFlow = useCallback(async () => {
    cancelDeviceFlow();
    setStatus("idle");
    setMessage("");
    setOauthPreparing(true);
    setOauthTimedOut(false);
    try {
      const response = await invoke<DeviceFlowStart>("start_oauth_flow", {
        provider: ideOrigin,
      });
      setDeviceFlow(response);
      loginIdRef.current = response.login_id;
      setOauthPreparing(false);
      startPolling(response.interval_seconds || 5);
    } catch (error) {
      setOauthPreparing(false);
      setStatus("error");
      setMessage("获取授权信息失败: " + String(error));
    }
  }, [cancelDeviceFlow, ideOrigin, setMessage, setStatus, startPolling]);

  const handleCopyUserCode = async () => {
    if (!deviceFlow?.user_code) return;
    await navigator.clipboard.writeText(deviceFlow.user_code).catch(() => {});
    setOauthUserCodeCopied(true);
    setTimeout(() => setOauthUserCodeCopied(false), 1500);
  };

  const handleCopyOAuthUrl = async () => {
    if (!deviceFlow?.verification_uri) return;
    await navigator.clipboard.writeText(deviceFlow.verification_uri).catch(() => {});
    setOauthUrlCopied(true);
    setTimeout(() => setOauthUrlCopied(false), 1500);
  };

  const handleOpenOAuthUrl = () => {
    if (deviceFlow?.verification_uri) {
      window.open(deviceFlow.verification_uri, "_blank");
    }
  };

  const handleTokenSubmit = async () => {
    const input = tokenInput.trim();
    if (!input) {
      setStatus("error");
      setMessage("请粘贴 Token 内容");
      return;
    }

    setStatus("loading");
    setMessage("正在解析 Token...");

    let tokens: string[] = [];
    const apiKeys: string[] = [];
    try {
      if (input.startsWith("[") && input.endsWith("]")) {
        const parsed = JSON.parse(input);
        if (Array.isArray(parsed)) {
          tokens = parsed
            .map((item: any) => item.refresh_token)
            .filter((token: any) => typeof token === "string" && token.startsWith("1//"));
          if (ideOrigin === "codex") {
            apiKeys.push(
              ...parsed
                .map((item: any) => item.openai_api_key || item.OPENAI_API_KEY || item.api_key)
                .filter(
                  (token: any) =>
                    typeof token === "string" && token.trim().startsWith("sk-")
                )
            );
          }
        }
      }
    } catch {}

    if (tokens.length === 0) {
      const matches = input.match(/1\/\/[a-zA-Z0-9_\-]+/g);
      if (matches) tokens = matches;
    }
    if (ideOrigin === "codex") {
      const keyMatches = input.match(/\bsk-[A-Za-z0-9_\-]+\b/g);
      if (keyMatches) apiKeys.push(...keyMatches);
    }

    tokens = [...new Set(tokens)];
    const uniqueApiKeys = [...new Set(apiKeys.map((item) => item.trim()).filter(Boolean))];
    if (tokens.length === 0 && uniqueApiKeys.length === 0) {
      tokens = [input];
    }

    const tokenAccounts: ScannedIdeAccount[] = tokens.map((token) => ({
      email: "",
      refresh_token: token.startsWith("1//") ? token : null,
      access_token: token.startsWith("1//") ? null : token,
      origin_platform: ideOrigin,
      source_path: "manual_paste",
    }));
    const apiKeyAccounts: ScannedIdeAccount[] = uniqueApiKeys.map((apiKey, index) => ({
      email: `codex-apikey-${index}@local`,
      refresh_token: null,
      access_token: null,
      origin_platform: "codex",
      source_path: "manual_paste",
      meta_json: JSON.stringify({
        auth_mode: "apikey",
        openai_api_key: apiKey,
        last_refresh: new Date().toISOString(),
      }),
    }));
    const accounts = ideOrigin === "codex" ? [...tokenAccounts, ...apiKeyAccounts] : tokenAccounts;

    if (accounts.length === 0) {
      setStatus("error");
      setMessage("未识别到可导入的 Token 或 API Key");
      return;
    }

    const summary = await importScannedAccounts(accounts, ideOrigin);
    presentImportSummary(summary, "导入完成", "所有账号导入失败，请检查 Token 格式");
  };

  const handlePickImportFiles = async () => {
    try {
      const selected = await openFileDialog({
        multiple: true,
        filters: [
          { name: "Import Files", extensions: ["json", "vscdb", "db"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
      const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
      if (paths.length === 0) return;

      setImporting(true);
      setStatus("loading");
      setMessage(`正在解析 ${paths.length} 个导入文件...`);

      const result = await invoke<FileImportScanResult>("import_from_files", { paths });
      const summary = await importScannedAccounts(result.accounts, ideOrigin);
      presentImportSummary(
        mergeFileImportSummary(summary, result, ideOrigin),
        "文件导入完成",
        "未找到可导入的账号数据"
      );
    } catch (error) {
      setStatus("error");
      setMessage("文件导入失败: " + String(error).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleScanLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在扫描本机 IDE 账号数据...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("scan_ide_accounts_from_local");
      setMessage(`发现 ${accounts.length} 个账号，正在导入...`);
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "本机导入完成", "未从本机找到可导入的账号数据");
    } catch (error) {
      setStatus("error");
      setMessage(String(error).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handlePickVscdb = async () => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        filters: [
          { name: "VSCode DB", extensions: ["vscdb", "db"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
      if (!selected || typeof selected !== "string") return;
      setImporting(true);
      setStatus("loading");
      setMessage("正在从数据库文件提取账号...");
      const accounts = await invoke<ScannedIdeAccount[]>("import_from_custom_db", {
        path: selected,
      });
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "数据库导入完成", "该文件中未找到可导入的账号数据");
    } catch (error) {
      setStatus("error");
      setMessage("导入失败: " + String(error));
    }
    setImporting(false);
  };

  const handleImportV1 = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在扫描旧版 v1 账号数据...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_v1_accounts");
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "旧版账号迁移完成", "未找到可迁移的旧版 v1 账号");
    } catch (error) {
      setStatus("error");
      setMessage(String(error).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportLocal = async (option: LocalImportOption) => {
    setImporting(true);
    setStatus("loading");
    setMessage(option.loadingMessage);
    try {
      const accounts = await invoke<ScannedIdeAccount[]>(option.command);
      const summary = await importScannedAccounts(accounts, option.fallbackOrigin);
      presentImportSummary(summary, option.successMessage, option.emptyMessage);
    } catch (error) {
      setStatus("error");
      setMessage(String(error).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  return {
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
  };
}
