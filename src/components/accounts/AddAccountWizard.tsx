import { useState, useEffect, useRef, useCallback } from "react";
import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import {
  Globe, Key, Upload, CheckCircle2, XCircle, Loader2,
  Copy, Check, Database, FileJson, ShieldCheck, RotateCw,
  ExternalLink, HardDrive, History, FolderOpen,
} from "lucide-react";
import { api } from "../../lib/api";
import { PLATFORM_LABELS } from "../../types";
import type { Platform } from "../../types";
import "./AddAccountWizard.css";

// ─────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────
type AccountMode = "api_key" | "sandbox";
type SandboxTab  = "oauth" | "token" | "import";
type Status      = "idle" | "loading" | "success" | "error";

interface Props { onClose: () => void; onSuccess: () => void; }

interface DeviceFlowStart {
  login_id:         string;
  user_code:        string;
  verification_uri: string;
  expires_in:       number;
  interval_seconds: number;
}

interface ScannedIdeAccount {
  email:           string;
  refresh_token:   string | null;
  access_token:    string | null;
  origin_platform: string;
  source_path:     string;
  meta_json?:      string | null;
  label?:          string | null;
}

interface ImportSummaryItem {
  label: string;
  origin_platform: string;
  source_path: string;
  reason?: string;
}

interface ImportSummary {
  ok: number;
  fail: number;
  successes: ImportSummaryItem[];
  failures: ImportSummaryItem[];
}

interface FileImportFailure {
  source_path: string;
  reason: string;
}

interface FileImportScanResult {
  accounts: ScannedIdeAccount[];
  failures: FileImportFailure[];
}

function parseMetaJson(metaJson?: string | null): Record<string, any> {
  if (!metaJson) return {};
  try {
    const value = JSON.parse(metaJson);
    return typeof value === "object" && value ? value : {};
  } catch {
    return {};
  }
}

function isCodexApiKeyAccount(acc: ScannedIdeAccount, origin: IdeOrigin) {
  if (origin !== "codex") return false;
  const meta = parseMetaJson(acc.meta_json);
  return meta?.auth_mode === "apikey" && typeof meta?.openai_api_key === "string" && meta.openai_api_key.trim().length > 0;
}

function basenameOfPath(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const segments = normalized.split("/");
  return segments[segments.length - 1] || path;
}

// ─────────────────────────────────────────────────────────────
// IDE 渠道来源列表
// ─────────────────────────────────────────────────────────────
const IDE_ORIGINS = [
  { value: "antigravity",    label: "Antigravity",    desc: "Claude OAuth 账号池（主渠道）" },
  { value: "claude_code",    label: "Claude Code",    desc: "Anthropic CLI 终端授权" },
  { value: "cursor",         label: "Cursor",         desc: "AI 编辑器 OAuth 账号" },
  { value: "windsurf",       label: "Windsurf",       desc: "Codeium 沙盒账号" },
  { value: "github_copilot", label: "GitHub Copilot", desc: "设备授权令牌" },
  { value: "claude_desktop", label: "Claude Desktop", desc: "桌面客户端凭证" },
  { value: "zed",            label: "Zed",            desc: "原生编辑器账号" },
  { value: "vscode",         label: "VS Code",        desc: "全球化存储库账号" },
  { value: "opencode",       label: "OpenCode",       desc: "沙盒授权账号" },
  { value: "codex",          label: "OpenAI Codex",   desc: "API 授权令牌" },
  { value: "kiro",           label: "Kiro",           desc: "第三方 IDE 账号" },
  { value: "gemini",         label: "Gemini CLI",     desc: "Google 授权账号" },
  { value: "codebuddy",      label: "CodeBuddy",      desc: "腾讯云 AI 助手" },
  { value: "codebuddy_cn",   label: "CodeBuddy CN",   desc: "腾讯云 AI 助手国区版" },
  { value: "workbuddy",      label: "WorkBuddy",      desc: "腾讯云国际版 AI 助手" },
  { value: "trae",           label: "Trae",           desc: "字节跳动 AI 编辑器" },
  { value: "qoder",          label: "Qoder",          desc: "Qoding 沙盒账号" },
  { value: "generic_ide",    label: "其它 / 通用",    desc: "自定义 OAuth 账号" },
] as const;

type IdeOrigin = typeof IDE_ORIGINS[number]["value"];

// ─────────────────────────────────────────────────────────────
// 渠道分类（与后端 services/oauth.rs 保持一致）
// ─────────────────────────────────────────────────────────────
/** B 类：Device Flow — 需要展示 user_code */
const DEVICE_FLOW_PROVIDERS: readonly string[] = ["github_copilot"];
/** C 类：只能文件导入，不支持 OAuth */
const IMPORT_ONLY_PROVIDERS: readonly string[] = ["claude_code", "claude_desktop", "vscode", "opencode", "generic_ide"];

const isDeviceFlow    = (p: string) => DEVICE_FLOW_PROVIDERS.includes(p);
const isImportOnly    = (p: string) => IMPORT_ONLY_PROVIDERS.includes(p);
const isBrowserOAuth  = (p: string) => !isDeviceFlow(p) && !isImportOnly(p);

// ─────────────────────────────────────────────────────────────
// StatusAlert helper
// ─────────────────────────────────────────────────────────────
function StatusAlert({ status, message }: { status: Status; message: string }) {
  if (status === "idle" || !message) return null;
  const map = {
    loading: { cls: "status-info",    icon: <Loader2 size={16} className="spin" /> },
    success: { cls: "status-success", icon: <CheckCircle2 size={16} /> },
    error:   { cls: "status-error",   icon: <XCircle size={16} /> },
  } as const;
  const { cls, icon } = map[status];
  return (
    <div className={`wiz-status ${cls}`}>
      {icon}
      <span>{message}</span>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// 导入扫描账号到 IDE 账号池
// ─────────────────────────────────────────────────────────────
async function importScannedAccounts(
  accounts: ScannedIdeAccount[],
  fallbackOrigin: IdeOrigin,
): Promise<ImportSummary> {
  let ok = 0, fail = 0;
  const successes: ImportSummaryItem[] = [];
  const failures: ImportSummaryItem[] = [];
  for (let i = 0; i < accounts.length; i++) {
    const acc = accounts[i];
    const origin = (acc.origin_platform || fallbackOrigin) as IdeOrigin;
    const hasRefresh = acc.refresh_token && acc.refresh_token.length > 0;
    const hasAccess  = acc.access_token  && acc.access_token.length  > 0;
    const isCodexApiKey = isCodexApiKeyAccount(acc, origin);
    const label = acc.label?.trim() || acc.email || `${origin}#${i + 1}`;
    if (!hasRefresh && !hasAccess && !isCodexApiKey) {
      fail++;
      failures.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
        reason: "缺少可导入的 access_token / refresh_token",
      });
      continue;
    }
    try {
      const meta = parseMetaJson(acc.meta_json);
      await api.ideAccounts.import([{
        id:              `scan-${Date.now()}-${i}`,
        email:           acc.email || (isCodexApiKey ? `codex-apikey-${i}@local` : `scan-${i}@local`),
        origin_platform: origin,
        token: {
          access_token:  hasAccess  ? acc.access_token!  : (isCodexApiKey ? "" : "requires_refresh"),
          refresh_token: hasRefresh ? acc.refresh_token! : "missing",
          expires_in:    3600,
          token_type:    "Bearer",
          updated_at:    new Date().toISOString(),
        },
        status:            "active",
        is_proxy_disabled: false,
        device_profile: {
          machine_id:    `sys-${Math.random().toString(36).substring(2, 10)}`,
          mac_machine_id:`mac-${Math.random().toString(36).substring(2, 10)}`,
          dev_device_id: crypto.randomUUID(),
          sqm_id:        `{${crypto.randomUUID()}}`,
        },
        created_at:  new Date().toISOString(),
        updated_at:  new Date().toISOString(),
        last_used:   new Date().toISOString(),
        meta_json:   Object.keys(meta).length > 0 ? JSON.stringify(meta) : null,
        label:       acc.label?.trim() || null,
      }]);
      ok++;
      successes.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
      });
    } catch (e) {
      fail++;
      failures.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
        reason: String(e),
      });
    }
    await new Promise(r => setTimeout(r, 60));
  }
  return { ok, fail, successes, failures };
}

// ─────────────────────────────────────────────────────────────
// 自定义高定下拉选择器 (取代原生 select)
// ─────────────────────────────────────────────────────────────
function CustomChannelSelect({ value, options, onChange }: { value: IdeOrigin; options: readonly { value: IdeOrigin; label: string; desc: string }[]; onChange: (v: IdeOrigin) => void }) {
  const [isOpen, setIsOpen] = useState(false);
  const selected = options.find(o => o.value === value) || options[0];

  return (
    <div className="wiz-custom-select-container">
      <div 
        className={`wiz-custom-select-trigger ${isOpen ? "open" : ""}`}
        onClick={() => setIsOpen(!isOpen)}
      >
        <div className="wiz-custom-select-val">
          <span className="wiz-cs-label">{selected.label}</span>
          <span className="wiz-cs-desc">{selected.desc}</span>
        </div>
        <div className="wiz-cs-arrow" />
      </div>

      {isOpen && (
        <>
          <div className="wiz-cs-overlay" onClick={() => setIsOpen(false)} />
          <div className="wiz-cs-menu">
            {options.map(o => (
              <div
                key={o.value}
                className={`wiz-cs-option ${o.value === value ? "selected" : ""}`}
                onClick={() => { onChange(o.value); setIsOpen(false); }}
              >
                <div className="wiz-cs-opt-label">{o.label}</div>
                <div className="wiz-cs-opt-desc">{o.desc}</div>
                {o.value === value && <Check size={14} className="wiz-cs-check" />}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// Main Component
// ─────────────────────────────────────────────────────────────
export default function AddAccountWizard({ onClose, onSuccess }: Props) {
  // ── 顶层模式 ─────────────────────────────────────────
  const [mode,       setMode]       = useState<AccountMode>("sandbox");
  const [sandboxTab, setSandboxTab] = useState<SandboxTab>("oauth");
  const [ideOrigin,  setIdeOrigin]  = useState<IdeOrigin>("antigravity");

  // ── 状态反馈 ─────────────────────────────────────────
  const [status,  setStatus]  = useState<Status>("idle");
  const [message, setMessage] = useState("");
  const [importSummary, setImportSummary] = useState<ImportSummary | null>(null);
  const resetStatus = () => { setStatus("idle"); setMessage(""); setImportSummary(null); };

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

  // ─────────────────────────────────────────────────────
  // API KEY 模式 state
  // ─────────────────────────────────────────────────────
  const [platform, setPlatform] = useState<Platform>("open_ai");
  const [keyName,  setKeyName]  = useState("");
  const [secret,   setSecret]   = useState("");
  const [baseUrl,  setBaseUrl]  = useState("");
  const [notes,    setNotes]    = useState("");

  const addKeyMut = useMutation({
    mutationFn: api.keys.add,
    onSuccess: () => {
      setStatus("success"); setMessage("API Key 已保存！");
      setTimeout(() => { onSuccess(); }, 1200);
    },
    onError: (e) => {
      setStatus("error"); setMessage("保存失败: " + String(e));
    },
  });

  const handleSaveApiKey = () => {
    if (!keyName.trim()) { setStatus("error"); setMessage("请填写标识名称"); return; }
    if (!secret.trim())  { setStatus("error"); setMessage("API Key 不能为空"); return; }
    setStatus("loading"); setMessage("正在保存...");
    addKeyMut.mutate({
      name:     keyName.trim(),
      platform,
      secret:   secret.trim(),
      base_url: platform === "custom" ? (baseUrl.trim() || undefined) : undefined,
      notes:    notes.trim() || undefined,
    });
  };

  // ─────────────────────────────────────────────────────
  // SANDBOX — Device Flow OAuth state
  // ─────────────────────────────────────────────────────
  const [deviceFlow,       setDeviceFlow]       = useState<DeviceFlowStart | null>(null);
  const [oauthUserCodeCopied, setOauthUserCodeCopied] = useState(false);
  const [oauthUrlCopied,   setOauthUrlCopied]   = useState(false);
  const [oauthPolling,     setOauthPolling]      = useState(false);
  const [oauthPreparing,   setOauthPreparing]    = useState(false);
  const [oauthTimedOut,    setOauthTimedOut]     = useState(false);

  // Refs 用于在 interval 回调中访问最新状态
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const loginIdRef      = useRef<string | null>(null);
  const sandboxTabRef   = useRef(sandboxTab);
  useEffect(() => { sandboxTabRef.current = sandboxTab; }, [sandboxTab]);

  // 清除轮询定时器
  const clearPoll = useCallback(() => {
    if (pollIntervalRef.current) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  // 取消当前 Device Flow（切换 tab / 关闭弹窗时）
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

  // 离开 OAuth Tab 时取消流程
  useEffect(() => {
    if (sandboxTab !== "oauth" || mode !== "sandbox") {
      cancelDeviceFlow();
    }
  }, [sandboxTab, mode, cancelDeviceFlow]);

  // 组件卸载时也取消
  useEffect(() => () => { cancelDeviceFlow(); }, [cancelDeviceFlow]);

  // 轮询函数
  const startPolling = useCallback((intervalSecs: number) => {
    clearPoll();
    setOauthPolling(true);
    pollIntervalRef.current = setInterval(async () => {
      if (!loginIdRef.current) { clearPoll(); return; }
      try {
        const result = await invoke<{
          done: boolean;
          token?: string;
          access_token?: string;
          refresh_token?: string;
          meta_json?: string | null;
          email?: string;
          name?: string;
          provider?: string;
        }>(
          "poll_oauth_login",
          { loginId: loginIdRef.current }
        );
        if (result.done && result.token) {
          clearPoll();
          setOauthPolling(false);
          const origin = ideOrigin;
          const accessToken =
            result.access_token
            || (origin === "gemini" || origin === "antigravity" ? "requires_refresh" : result.token);
          const refreshToken =
            result.refresh_token
            || (origin === "gemini" || origin === "antigravity" ? result.token : "missing");
          // 优先使用后端返回的真实 email，否则用渠道+时间戳占位
          const accountEmail = result.email
            || `${origin}-${Date.now()}@oauth.local`;
          await api.ideAccounts.import([{
            id:              `oauth-${Date.now()}`,
            email:           accountEmail,
            origin_platform: origin,
            token: {
              access_token:  accessToken,
              refresh_token: refreshToken,
              expires_in:    3600,
              token_type:    "Bearer",
              updated_at:    new Date().toISOString(),
            },
            status:            "active",
            is_proxy_disabled: false,
            device_profile: {
              machine_id:    `sys-${Math.random().toString(36).substring(2, 10)}`,
              mac_machine_id:`mac-${Math.random().toString(36).substring(2, 10)}`,
              dev_device_id: crypto.randomUUID(),
              sqm_id:        `{${crypto.randomUUID()}}`,
            },
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
            last_used:  new Date().toISOString(),
            meta_json: result.meta_json || null,
          }]);
          setStatus("success");
          const displayEmail = result.email ? `（${result.email}）` : "";
          setMessage(`OAuth 授权成功！账号已导入${displayEmail}。`);
          loginIdRef.current = null;
          setTimeout(() => onSuccess(), 1200);
        }
      } catch (e) {
        clearPoll();
        setOauthPolling(false);
        const msg = String(e);
        if (msg.includes("过期")) {
          setOauthTimedOut(true);
          setStatus("error");
          setMessage("授权码已过期，请点击「重新获取」");
        } else if (!msg.includes("取消")) {
          setStatus("error");
          setMessage("授权失败: " + msg);
        }
      }
    }, intervalSecs * 1000);
  }, [clearPoll, ideOrigin, onSuccess]);

  // 启动 Device Flow
  const handleStartDeviceFlow = useCallback(async () => {
    cancelDeviceFlow();
    resetStatus();
    setOauthPreparing(true);
    setOauthTimedOut(false);
    try {
      const resp = await invoke<DeviceFlowStart>("start_oauth_flow", {
        provider: ideOrigin,
      });
      setDeviceFlow(resp);
      loginIdRef.current = resp.login_id;
      setOauthPreparing(false);
      startPolling(resp.interval_seconds || 5);
    } catch (e) {
      setOauthPreparing(false);
      setStatus("error");
      setMessage("获取授权信息失败: " + String(e));
    }
  }, [cancelDeviceFlow, ideOrigin, startPolling]);

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

  // ─────────────────────────────────────────────────────
  // SANDBOX — Token Tab state
  // ─────────────────────────────────────────────────────
  const [tokenInput, setTokenInput] = useState("");

  const handleTokenSubmit = async () => {
    const input = tokenInput.trim();
    if (!input) { setStatus("error"); setMessage("请粘贴 Token 内容"); return; }

    setStatus("loading"); setMessage("正在解析 Token...");

    let tokens: string[] = [];
    const apiKeys: string[] = [];
    try {
      if (input.startsWith("[") && input.endsWith("]")) {
        const parsed = JSON.parse(input);
        if (Array.isArray(parsed)) {
          tokens = parsed
            .map((x: any) => x.refresh_token)
            .filter((t: any) => typeof t === "string" && t.startsWith("1//"));
          if (ideOrigin === "codex") {
            apiKeys.push(
              ...parsed
                .map((x: any) => x.openai_api_key || x.OPENAI_API_KEY || x.api_key)
                .filter((t: any) => typeof t === "string" && t.trim().startsWith("sk-"))
            );
          }
        }
      }
    } catch { /* fallback */ }

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
    if (tokens.length === 0 && uniqueApiKeys.length === 0) tokens = [input];

    const tokenAccounts: ScannedIdeAccount[] = tokens.map(t => ({
      email:           "",
      refresh_token:   t.startsWith("1//") ? t : null,
      access_token:    t.startsWith("1//") ? null : t,
      origin_platform: ideOrigin,
      source_path:     "manual_paste",
    }));
    const apiKeyAccounts: ScannedIdeAccount[] = uniqueApiKeys.map((apiKey, index) => ({
      email:           `codex-apikey-${index}@local`,
      refresh_token:   null,
      access_token:    null,
      origin_platform: "codex",
      source_path:     "manual_paste",
      meta_json:       JSON.stringify({
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

  // ─────────────────────────────────────────────────────
  // SANDBOX — Import Tab state
  // ─────────────────────────────────────────────────────
  const [importing, setImporting] = useState(false);

  const handlePickImportFiles = async () => {
    try {
      const selected = await openFileDialog({
        multiple: true,
        filters: [
          { name: "Import Files", extensions: ["json", "vscdb", "db"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
      const paths = Array.isArray(selected) ? selected : (selected ? [selected] : []);
      if (paths.length === 0) return;

      setImporting(true);
      setStatus("loading");
      setMessage(`正在解析 ${paths.length} 个导入文件...`);

      const result = await invoke<FileImportScanResult>("import_from_files", { paths });
      const summary = await importScannedAccounts(result.accounts, ideOrigin);
      const mergedSummary: ImportSummary = {
        ...summary,
        fail: summary.fail + result.failures.length,
        failures: [
          ...summary.failures,
          ...result.failures.map((item) => ({
            label: basenameOfPath(item.source_path),
            origin_platform: ideOrigin,
            source_path: item.source_path,
            reason: item.reason,
          })),
        ],
      };
      presentImportSummary(mergedSummary, "文件导入完成", "未找到可导入的账号数据");
    } catch (e) {
      setStatus("error");
      setMessage("文件导入失败: " + String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  // 自动扫描本机 IDE
  const handleScanLocal = async () => {
    setImporting(true);
    setStatus("loading"); setMessage("正在扫描本机 IDE 账号数据...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("scan_ide_accounts_from_local");
      setMessage(`发现 ${accounts.length} 个账号，正在导入...`);
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "本机导入完成", "未从本机找到可导入的账号数据");
    } catch (e) {
      setStatus("error"); setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  // 选择 .vscdb 文件
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
      setStatus("loading"); setMessage("正在从数据库文件提取账号...");
      const accounts = await invoke<ScannedIdeAccount[]>("import_from_custom_db", { path: selected });
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "数据库导入完成", "该文件中未找到可导入的账号数据");
    } catch (e) {
      setStatus("error"); setMessage("导入失败: " + String(e));
    }
    setImporting(false);
  };

  // 旧版 v1 迁移
  const handleImportV1 = async () => {
    setImporting(true);
    setStatus("loading"); setMessage("正在扫描旧版 v1 账号数据...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_v1_accounts");
      const summary = await importScannedAccounts(accounts, ideOrigin);
      presentImportSummary(summary, "旧版账号迁移完成", "未找到可迁移的旧版 v1 账号");
    } catch (e) {
      setStatus("error"); setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportGeminiLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Gemini 登录信息...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_gemini_from_local");
      const summary = await importScannedAccounts(accounts, "gemini");
      presentImportSummary(summary, "本地 Gemini 账号导入完成", "未读取到可导入的本地 Gemini 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportCodexLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Codex 登录信息...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_codex_from_local");
      const summary = await importScannedAccounts(accounts, "codex");
      presentImportSummary(summary, "本地 Codex 账号导入完成", "未读取到可导入的本地 Codex 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportKiroLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Kiro 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_kiro_from_local");
      const summary = await importScannedAccounts(accounts, "kiro");
      presentImportSummary(summary, "本地 Kiro 账号导入完成", "未读取到可导入的本地 Kiro 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportCursorLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Cursor 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_cursor_from_local");
      const summary = await importScannedAccounts(accounts, "cursor");
      presentImportSummary(summary, "本地 Cursor 账号导入完成", "未读取到可导入的本地 Cursor 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportWindsurfLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Windsurf 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_windsurf_from_local");
      const summary = await importScannedAccounts(accounts, "windsurf");
      presentImportSummary(summary, "本地 Windsurf 账号导入完成", "未读取到可导入的本地 Windsurf 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportCodeBuddyLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 CodeBuddy 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_codebuddy_from_local");
      const summary = await importScannedAccounts(accounts, "codebuddy");
      presentImportSummary(summary, "本地 CodeBuddy 账号导入完成", "未读取到可导入的本地 CodeBuddy 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportCodeBuddyCnLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 CodeBuddy CN 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_codebuddy_cn_from_local");
      const summary = await importScannedAccounts(accounts, "codebuddy_cn");
      presentImportSummary(summary, "本地 CodeBuddy CN 账号导入完成", "未读取到可导入的本地 CodeBuddy CN 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportWorkBuddyLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 WorkBuddy 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_workbuddy_from_local");
      const summary = await importScannedAccounts(accounts, "workbuddy");
      presentImportSummary(summary, "本地 WorkBuddy 账号导入完成", "未读取到可导入的本地 WorkBuddy 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportZedLocal = async () => {
    setImporting(true);
    setStatus("loading");
    setMessage("正在读取本地 Zed 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_zed_from_local");
      const summary = await importScannedAccounts(accounts, "zed");
      presentImportSummary(summary, "本地 Zed 账号导入完成", "未读取到可导入的本地 Zed 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportQoderLocal = async () => {
    setImporting(true);
    setStatus("loading"); setMessage("正在读取本地 Qoder 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_qoder_from_local");
      const summary = await importScannedAccounts(accounts, "qoder");
      presentImportSummary(summary, "本地 Qoder 账号导入完成", "未读取到可导入的本地 Qoder 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  const handleImportTraeLocal = async () => {
    setImporting(true);
    setStatus("loading"); setMessage("正在读取本地 Trae 登录...");
    try {
      const accounts = await invoke<ScannedIdeAccount[]>("import_trae_from_local");
      const summary = await importScannedAccounts(accounts, "trae");
      presentImportSummary(summary, "本地 Trae 账号导入完成", "未读取到可导入的本地 Trae 账号");
    } catch (e) {
      setStatus("error");
      setMessage(String(e).replace(/^Error:\s*/, ""));
    }
    setImporting(false);
  };

  // ─────────────────────────────────────────────────────
  // Tab 切换
  // ─────────────────────────────────────────────────────
  const handleTabChange = (tab: SandboxTab) => {
    setSandboxTab(tab);
    resetStatus();
    setTokenInput("");
    if (tab === "oauth" && isImportOnly(ideOrigin)) {
      setIdeOrigin("antigravity");
    }
  };

  // mode 切换
  const handleModeChange = (m: AccountMode) => {
    setMode(m);
    resetStatus();
    setSandboxTab("oauth");
  };

  // ─────────────────────────────────────────────────────
  // Render
  // ─────────────────────────────────────────────────────
  return (
    <div className="wiz-overlay" onClick={onClose}>
      <div className="wiz-panel" onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div className="wiz-header">
          <div className="wiz-header-title">
            <ShieldCheck size={20} />
            <span>添加账号</span>
          </div>
          <button className="wiz-close-btn" onClick={onClose}>✕</button>
        </div>

        {/* 模式选择 */}
        <div className="wiz-mode-row">
          <button
            className={`wiz-mode-btn ${mode === "sandbox" ? "active" : ""}`}
            onClick={() => handleModeChange("sandbox")}
          >
            <Globe size={16} />
            <span>沙盒账号（IDE / OAuth）</span>
          </button>
          <button
            className={`wiz-mode-btn ${mode === "api_key" ? "active" : ""}`}
            onClick={() => handleModeChange("api_key")}
          >
            <Key size={16} />
            <span>API 密钥（Cloud）</span>
          </button>
        </div>

        {/* ────────────── SANDBOX 模式 ────────────── */}
        {mode === "sandbox" && (
          <>
            {/* 渠道来源选择器 */}
            <div className="wiz-channel-row">
              <label className="wiz-channel-label">渠道来源</label>
              <CustomChannelSelect 
                value={ideOrigin} 
                options={sandboxTab === "oauth" ? IDE_ORIGINS.filter(o => !isImportOnly(o.value)) : IDE_ORIGINS}
                onChange={setIdeOrigin} 
              />
            </div>
            {/* Tab 导航 */}
            <div className="wiz-tabs">
              {(["oauth", "token", "import"] as SandboxTab[]).map(tab => {
                const labels = { oauth: "OAuth 授权", token: "Token 粘贴", import: "导入账号" };
                const icons  = { oauth: <Globe size={14} />, token: <Key size={14} />, import: <Database size={14} /> };
                return (
                  <button
                    key={tab}
                    className={`wiz-tab ${sandboxTab === tab ? "active" : ""}`}
                    onClick={() => handleTabChange(tab)}
                  >
                    {icons[tab]}{labels[tab]}
                  </button>
                );
              })}
            </div>

            <StatusAlert status={status} message={message} />
            {importSummary && (
              <div className="wiz-import-summary">
                <div className="wiz-import-summary-head">
                  <strong>导入结果汇总</strong>
                  <span>成功 {importSummary.ok} / 失败 {importSummary.fail}</span>
                </div>
                {importSummary.failures.length > 0 && (
                  <div className="wiz-import-summary-list">
                    {importSummary.failures.slice(0, 8).map((item, index) => (
                      <div key={`${item.label}-${index}`} className="wiz-import-summary-item failure">
                        <div className="wiz-import-summary-title">{item.label} · {item.origin_platform}</div>
                        <div className="wiz-import-summary-meta">{item.source_path}</div>
                        <div className="wiz-import-summary-reason">{item.reason || "未知错误"}</div>
                      </div>
                    ))}
                    {importSummary.failures.length > 8 && (
                      <div className="wiz-import-summary-more">其余 {importSummary.failures.length - 8} 条失败记录已省略显示。</div>
                    )}
                  </div>
                )}
                {importSummary.ok > 0 && importSummary.fail === 0 && (
                  <div className="wiz-import-summary-success">所有账号已成功导入。</div>
                )}
              </div>
            )}

            {/* ── OAuth Tab (Device Flow) ── */}
            {sandboxTab === "oauth" && (
              <div className="wiz-tab-content">
                {!deviceFlow && !oauthPreparing && (
                  <div className="wiz-oauth-empty">

                    {/* C 类：仅支持导入 */}
                    {isImportOnly(ideOrigin) && (
                      <>
                        <ShieldCheck size={40} className="wiz-oauth-icon" style={{ color: "var(--warning, #f59e0b)" }} />
                        <p className="wiz-oauth-desc">
                          <strong>{IDE_ORIGINS.find(o => o.value === ideOrigin)?.label}</strong> 渠道不支持 OAuth 授权流程。<br />
                          请切换到「Token 粘贴」或「导入账号」Tab 导入凭证。
                        </p>
                        <button className="wiz-btn-ghost" onClick={() => handleTabChange("import")}>
                          <Database size={16} /> 前往导入 Tab
                        </button>
                      </>
                    )}

                    {/* A 类：浏览器自动回调 */}
                    {isBrowserOAuth(ideOrigin) && (
                      <>
                        <Globe size={40} className="wiz-oauth-icon" />
                        <p className="wiz-oauth-desc">
                          点击下方按钮，将自动打开浏览器进行 OAuth 授权。<br />
                          授权完成后浏览器页面会自动关闭，账号将自动导入。
                        </p>
                        <button
                          className="wiz-btn-primary"
                          onClick={handleStartDeviceFlow}
                          disabled={status === "success"}
                        >
                          <Globe size={16} /> 开启 OAuth 授权
                        </button>
                      </>
                    )}

                    {/* B 类：Device Flow */}
                    {isDeviceFlow(ideOrigin) && (
                      <>
                        <Key size={40} className="wiz-oauth-icon" />
                        <p className="wiz-oauth-desc">
                          点击下方按钮获取授权码，然后在弹出的授权页面中输入验证码完成授权。
                        </p>
                        <button
                          className="wiz-btn-primary"
                          onClick={handleStartDeviceFlow}
                          disabled={status === "success"}
                        >
                          <Key size={16} /> 获取授权验证码
                        </button>
                      </>
                    )}

                  </div>
                )}

                {oauthPreparing && (
                  <div className="wiz-oauth-empty">
                    <Loader2 size={32} className="spin wiz-oauth-icon" />
                    <p className="wiz-oauth-desc">正在获取验证码...</p>
                  </div>
                )}

                {deviceFlow && !oauthPreparing && (
                  <div className="wiz-device-flow">

                    {/* B 类（github_copilot）：展示 user_code + 跳转链接 */}
                    {deviceFlow.user_code && (
                      <div className="wiz-user-code-block">
                        <p className="wiz-uc-label">在授权页面输入此验证码：</p>
                        <div className="wiz-user-code">
                          {deviceFlow.user_code}
                          <button className="wiz-copy-code-btn" onClick={handleCopyUserCode} title="复制验证码">
                            {oauthUserCodeCopied ? <Check size={14} /> : <Copy size={14} />}
                          </button>
                        </div>
                      </div>
                    )}

                    {/* 验证链接（所有渠道都显示） */}
                    <div className="wiz-verification-url">
                      <p className="wiz-uc-label">
                        {deviceFlow.user_code ? "授权链接：" : "正在等待浏览器授权回调..."}
                      </p>
                      <div className="wiz-url-row">
                        <code className="wiz-url-text">{deviceFlow.verification_uri}</code>
                        <button className="wiz-icon-btn" onClick={handleCopyOAuthUrl} title="复制链接">
                          {oauthUrlCopied ? <Check size={13} /> : <Copy size={13} />}
                        </button>
                        <button className="wiz-icon-btn" onClick={handleOpenOAuthUrl} title="在浏览器中打开">
                          <ExternalLink size={13} />
                        </button>
                      </div>
                    </div>

                    {/* 轮询状态 */}
                    <div className="wiz-poll-status">
                      {oauthPolling && !oauthTimedOut && (
                        <div className="wiz-poll-row">
                          <div className="wiz-poll-pulse" />
                          <span>
                            {deviceFlow.user_code
                              ? "等待您在浏览器中输入验证码并完成授权..."
                              : "等待浏览器授权回调，完成后将自动导入..."
                            }
                          </span>
                        </div>
                      )}
                      {oauthTimedOut && (
                        <div className="wiz-poll-row error">
                          <XCircle size={14} />
                          <span>授权已超时</span>
                          <button className="wiz-link-btn" onClick={handleStartDeviceFlow}>
                            <RotateCw size={12} /> 重新发起
                          </button>
                        </div>
                      )}
                    </div>

                    {!oauthTimedOut && (
                      <button
                        className="wiz-btn-ghost wiz-retry-btn"
                        onClick={handleStartDeviceFlow}
                        disabled={oauthPreparing}
                      >
                        <RotateCw size={14} /> 重新发起授权
                      </button>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* ── Token Tab ── */}
            {sandboxTab === "token" && (
              <div className="wiz-tab-content">
                <p className="wiz-field-hint">
                  支持单个 Token、批量 JSON 数组，或包含多个 <code>1//xxx</code> 的文本。
                  {ideOrigin === "codex" ? <> 也支持直接粘贴 <code>sk-...</code> 作为 Codex API Key 账号导入。</> : null}
                </p>
                <textarea
                  className="wiz-textarea"
                  placeholder={`粘贴 Token 内容，例如：\n1//xxxxxxxx...\n或 JSON 数组 [{\"refresh_token\": \"1//xxx\"}]`}
                  value={tokenInput}
                  onChange={e => setTokenInput(e.target.value)}
                  rows={7}
                  disabled={status === "loading" || status === "success"}
                />
                <button
                  className="wiz-btn-primary"
                  onClick={handleTokenSubmit}
                  disabled={status === "loading" || status === "success" || !tokenInput.trim()}
                >
                  {status === "loading" ? <Loader2 size={16} className="spin" /> : <Upload size={16} />}
                  批量导入
                </button>
              </div>
            )}

            {/* ── Import Tab ── */}
            {sandboxTab === "import" && (
              <div className="wiz-tab-content wiz-import-tab">

                {/* 方案 A：本机自动扫描 */}
                <div className="wiz-import-section">
                  <h4 className="wiz-import-section-title">
                    <HardDrive size={15} /> 方案 A — 从本机 IDE 自动提取
                  </h4>
                  <p className="wiz-import-desc">
                    自动扫描本机已安装 IDE（VS Code / Cursor / Windsurf / Kiro 等）的账号存储，一键导入。
                  </p>
                  <button
                    className="wiz-import-btn"
                    onClick={handleScanLocal}
                    disabled={importing || status === "success"}
                  >
                    {importing ? <Loader2 size={15} className="spin" /> : <HardDrive size={15} />}
                    一键扫描本机 IDE 账号
                  </button>

                  <button
                    className="wiz-import-btn secondary"
                    onClick={handlePickVscdb}
                    disabled={importing || status === "success"}
                  >
                    <FolderOpen size={15} />
                    选择 .vscdb 文件导入
                  </button>
                </div>

                <div className="wiz-import-divider">或</div>

                {ideOrigin === "gemini" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Gemini 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>~/.gemini/oauth_creds.json</code> 与 <code>google_accounts.json</code>。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportGeminiLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Gemini 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "codex" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Codex 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>~/.codex/auth.json</code>。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportCodexLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Codex 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "kiro" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Kiro 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>~/.aws/sso/cache/kiro-auth-token.json</code> 与本地 <code>profile.json</code>。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportKiroLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Kiro 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "cursor" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Cursor 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>Cursor/User/globalStorage/state.vscdb</code> 中的登录态。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportCursorLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Cursor 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "windsurf" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Windsurf 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>Windsurf/User/globalStorage/state.vscdb</code> 中的登录态。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportWindsurfLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Windsurf 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "codebuddy" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 CodeBuddy 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>CodeBuddy/User/globalStorage/state.vscdb</code> 中的登录态。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportCodeBuddyLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 CodeBuddy 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "codebuddy_cn" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 CodeBuddy CN 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>CodeBuddy CN/User/globalStorage/state.vscdb</code> 中的登录态。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportCodeBuddyCnLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 CodeBuddy CN 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "workbuddy" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 WorkBuddy 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>WorkBuddy/User/globalStorage/state.vscdb</code> 中的登录态。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportWorkBuddyLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 WorkBuddy 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "zed" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Zed 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前系统中的 <code>Zed Keychain</code> 登录信息。该能力当前仅在 macOS 上可用。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportZedLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Zed 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "qoder" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Qoder 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>Qoder/User/globalStorage/state.vscdb</code> 中的本地登录信息。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportQoderLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Qoder 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {ideOrigin === "trae" && (
                  <>
                    <div className="wiz-import-section">
                      <h4 className="wiz-import-section-title">
                        <Globe size={15} /> 方案 B — 导入本地 Trae 登录
                      </h4>
                      <p className="wiz-import-desc">
                        直接读取当前用户目录下的 <code>Trae/User/globalStorage/storage.json</code>。
                      </p>
                      <button
                        className="wiz-import-btn"
                        onClick={handleImportTraeLocal}
                        disabled={importing || status === "success"}
                      >
                        <Globe size={15} />
                        导入本地 Trae 登录
                      </button>
                    </div>

                    <div className="wiz-import-divider">或</div>
                  </>
                )}

                {/* 方案 B：JSON 文件 */}
                <div className="wiz-import-section">
                  <h4 className="wiz-import-section-title">
                    <FileJson size={15} /> 方案 B — 导入文件
                  </h4>
                  <p className="wiz-import-desc">
                    支持选择一个或多个文件，兼容 <code>.json</code>、<code>.vscdb</code>、<code>auth.json</code>、
                    <code>oauth_creds.json</code> 等常见导入格式。
                  </p>
                  <button
                    className="wiz-import-btn"
                    onClick={handlePickImportFiles}
                    disabled={importing || status === "success"}
                  >
                    <FileJson size={15} />
                    选择文件导入
                  </button>
                </div>

                <div className="wiz-import-divider">或</div>

                {/* 方案 C：旧版迁移 */}
                <div className="wiz-import-section">
                  <h4 className="wiz-import-section-title">
                    <History size={15} /> 方案 C — 旧版 v1 账号迁移
                  </h4>
                  <p className="wiz-import-desc">
                    从旧版 AI Singularity / Antigravity 的本地存储迁移历史账号数据。
                  </p>
                  <button
                    className="wiz-import-btn"
                    onClick={handleImportV1}
                    disabled={importing || status === "success"}
                  >
                    <History size={15} />
                    迁移旧版 v1 账号
                  </button>
                </div>

              </div>
            )}
          </>
        )}

        {/* ────────────── API KEY 模式 ────────────── */}
        {mode === "api_key" && (
          <div className="wiz-tab-content">
            <StatusAlert status={status} message={message} />
            {importSummary && (
              <div className="wiz-import-summary">
                <div className="wiz-import-summary-head">
                  <strong>导入结果汇总</strong>
                  <span>成功 {importSummary.ok} / 失败 {importSummary.fail}</span>
                </div>
                {importSummary.failures.length > 0 && (
                  <div className="wiz-import-summary-list">
                    {importSummary.failures.slice(0, 8).map((item, index) => (
                      <div key={`${item.label}-${index}`} className="wiz-import-summary-item failure">
                        <div className="wiz-import-summary-title">{item.label} · {item.origin_platform}</div>
                        <div className="wiz-import-summary-meta">{item.source_path}</div>
                        <div className="wiz-import-summary-reason">{item.reason || "未知错误"}</div>
                      </div>
                    ))}
                    {importSummary.failures.length > 8 && (
                      <div className="wiz-import-summary-more">其余 {importSummary.failures.length - 8} 条失败记录已省略显示。</div>
                    )}
                  </div>
                )}
                {importSummary.ok > 0 && importSummary.fail === 0 && (
                  <div className="wiz-import-summary-success">所有账号已成功导入。</div>
                )}
              </div>
            )}

            <div className="wiz-field-row">
              <label className="wiz-field-label">Provider</label>
              <select className="wiz-select" value={platform} onChange={e => setPlatform(e.target.value as Platform)}>
                {Object.entries(PLATFORM_LABELS).map(([k, v]) => (
                  <option key={k} value={k}>{v as string}</option>
                ))}
              </select>
            </div>

            <div className="wiz-field-row">
              <label className="wiz-field-label">标识名称</label>
              <input className="wiz-input" placeholder="例如：主账户-GPT4o" value={keyName} onChange={e => setKeyName(e.target.value)} />
            </div>

            <div className="wiz-field-row">
              <label className="wiz-field-label">API Key</label>
              <input className="wiz-input" type="password" placeholder="sk-..." value={secret} onChange={e => setSecret(e.target.value)} />
            </div>

            {platform === "custom" && (
              <div className="wiz-field-row">
                <label className="wiz-field-label">Base URL</label>
                <input className="wiz-input" placeholder="https://api.example.com/v1" value={baseUrl} onChange={e => setBaseUrl(e.target.value)} />
              </div>
            )}

            <div className="wiz-field-row">
              <label className="wiz-field-label">备注</label>
              <input className="wiz-input" placeholder="可选" value={notes} onChange={e => setNotes(e.target.value)} />
            </div>

            <div className="wiz-field-hint">
              <ShieldCheck size={13} /> Key 仅存储于本地，不上传任何服务器
            </div>

            <button
              className="wiz-btn-primary"
              onClick={handleSaveApiKey}
              disabled={status === "loading" || status === "success"}
            >
              {status === "loading" ? <Loader2 size={16} className="spin" /> : <Key size={16} />}
              保存 API Key
            </button>
          </div>
        )}

      </div>
    </div>
  );
}
