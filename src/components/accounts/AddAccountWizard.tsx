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
  { value: "trae",           label: "Trae",           desc: "字节跳动 AI 编辑器" },
  { value: "qoder",          label: "Qoder",          desc: "Qoding 沙盒账号" },
  { value: "generic_ide",    label: "其它 / 通用",    desc: "自定义 OAuth 账号" },
] as const;

type IdeOrigin = typeof IDE_ORIGINS[number]["value"];

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
): Promise<{ ok: number; fail: number }> {
  let ok = 0, fail = 0;
  for (let i = 0; i < accounts.length; i++) {
    const acc = accounts[i];
    const origin = (acc.origin_platform || fallbackOrigin) as IdeOrigin;
    const hasRefresh = acc.refresh_token && acc.refresh_token.length > 0;
    const hasAccess  = acc.access_token  && acc.access_token.length  > 0;
    if (!hasRefresh && !hasAccess) { fail++; continue; }
    try {
      await api.ideAccounts.import([{
        id:              `scan-${Date.now()}-${i}`,
        email:           acc.email || `scan-${i}@local`,
        origin_platform: origin,
        token: {
          access_token:  hasAccess  ? acc.access_token!  : "requires_refresh",
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
      }]);
      ok++;
    } catch { fail++; }
    await new Promise(r => setTimeout(r, 60));
  }
  return { ok, fail };
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
  const resetStatus = () => { setStatus("idle"); setMessage(""); };

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
        const result = await invoke<{ done: boolean; token?: string }>(
          "poll_oauth_login",
          { loginId: loginIdRef.current }
        );
        if (result.done && result.token) {
          clearPoll();
          setOauthPolling(false);
          // 授权成功后把 token 作为 access_token 导入
          const origin = ideOrigin;
          await api.ideAccounts.import([{
            id:              `oauth-${Date.now()}`,
            email:           `oauth-${Date.now()}@local`,
            origin_platform: origin,
            token: {
              access_token:  result.token,
              refresh_token: "oauth_imported",
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
          }]);
          setStatus("success");
          setMessage("OAuth 授权成功！账号已导入。");
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
    try {
      if (input.startsWith("[") && input.endsWith("]")) {
        const parsed = JSON.parse(input);
        if (Array.isArray(parsed)) {
          tokens = parsed
            .map((x: any) => x.refresh_token)
            .filter((t: any) => typeof t === "string" && t.startsWith("1//"));
        }
      }
    } catch { /* fallback */ }

    if (tokens.length === 0) {
      const matches = input.match(/1\/\/[a-zA-Z0-9_\-]+/g);
      if (matches) tokens = matches;
    }
    tokens = [...new Set(tokens)];
    if (tokens.length === 0) tokens = [input];

    const accounts: ScannedIdeAccount[] = tokens.map(t => ({
      email:           "",
      refresh_token:   t.startsWith("1//") ? t : null,
      access_token:    t.startsWith("1//") ? null : t,
      origin_platform: ideOrigin,
      source_path:     "manual_paste",
    }));

    const { ok, fail } = await importScannedAccounts(accounts, ideOrigin);

    if (ok === tokens.length) {
      setStatus("success"); setMessage(`成功导入 ${ok} 个账号！`);
      setTimeout(() => onSuccess(), 1200);
    } else if (ok > 0) {
      setStatus("success"); setMessage(`导入完成：成功 ${ok} 个，失败 ${fail} 个`);
    } else {
      setStatus("error"); setMessage("所有账号导入失败，请检查 Token 格式");
    }
  };

  // ─────────────────────────────────────────────────────
  // SANDBOX — Import Tab state
  // ─────────────────────────────────────────────────────
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [importing, setImporting] = useState(false);

  // JSON 文件导入
  const handleImportJsonFile = async (file: File) => {
    setImporting(true);
    setStatus("loading"); setMessage("正在解析文件...");
    try {
      const content = await file.text();
      const parsed = JSON.parse(content);
      const arr = Array.isArray(parsed) ? parsed : [parsed];
      const accounts: ScannedIdeAccount[] = arr.map((item: any) => ({
        email:           item.email || "",
        refresh_token:   item.refresh_token || item.refreshToken || null,
        access_token:    item.access_token  || item.accessToken  || item.token || null,
        origin_platform: item.origin_platform || item.platform || ideOrigin,
        source_path:     "file_import",
      }));
      const { ok, fail } = await importScannedAccounts(accounts, ideOrigin);
      if (ok > 0) {
        setStatus("success"); setMessage(`成功导入 ${ok} 个账号${fail > 0 ? `，失败 ${fail} 个` : ""}！`);
        if (fail === 0) setTimeout(() => onSuccess(), 1200);
      } else {
        setStatus("error"); setMessage("未找到可导入的账号数据");
      }
    } catch (e) {
      setStatus("error"); setMessage("文件解析失败: " + String(e));
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
      const { ok, fail } = await importScannedAccounts(accounts, ideOrigin);
      if (ok > 0) {
        setStatus("success"); setMessage(`成功从本机导入 ${ok} 个账号${fail > 0 ? `，失败 ${fail} 个` : ""}！`);
        if (fail === 0) setTimeout(() => onSuccess(), 1200);
      } else {
        setStatus("error"); setMessage("未从本机找到可导入的账号数据");
      }
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
      const { ok, fail } = await importScannedAccounts(accounts, ideOrigin);
      if (ok > 0) {
        setStatus("success"); setMessage(`成功导入 ${ok} 个账号${fail > 0 ? `，失败 ${fail} 个` : ""}！`);
        if (fail === 0) setTimeout(() => onSuccess(), 1200);
      } else {
        setStatus("error"); setMessage("该文件中未找到可导入的账号数据");
      }
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
      const { ok, fail } = await importScannedAccounts(accounts, ideOrigin);
      if (ok > 0) {
        setStatus("success"); setMessage(`旧版账号迁移完成：成功 ${ok} 个${fail > 0 ? `，失败 ${fail} 个` : ""}！`);
        if (fail === 0) setTimeout(() => onSuccess(), 1200);
      } else {
        setStatus("error"); setMessage("未找到可迁移的旧版 v1 账号");
      }
    } catch (e) {
      setStatus("error"); setMessage(String(e).replace(/^Error:\s*/, ""));
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

        {/* 渠道来源选择器 */}
        <div className="wiz-channel-row">
          <label className="wiz-channel-label">渠道来源</label>
          <select
            className="wiz-channel-select"
            value={ideOrigin}
            onChange={e => setIdeOrigin(e.target.value as IdeOrigin)}
          >
            {IDE_ORIGINS.map(o => (
              <option key={o.value} value={o.value}>{o.label} — {o.desc}</option>
            ))}
          </select>
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

            {/* ── OAuth Tab (Device Flow) ── */}
            {sandboxTab === "oauth" && (
              <div className="wiz-tab-content">
                {!deviceFlow && !oauthPreparing && (
                  <div className="wiz-oauth-empty">
                    <Globe size={40} className="wiz-oauth-icon" />
                    <p className="wiz-oauth-desc">
                      使用 Device Flow 授权，无需回调端口。点击下方按钮获取验证码。
                    </p>
                    <button
                      className="wiz-btn-primary"
                      onClick={handleStartDeviceFlow}
                      disabled={status === "success"}
                    >
                      <Globe size={16} /> 获取授权验证码
                    </button>
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
                    {/* 用户验证码 */}
                    <div className="wiz-user-code-block">
                      <p className="wiz-uc-label">在浏览器中输入此验证码：</p>
                      <div className="wiz-user-code">
                        {deviceFlow.user_code}
                        <button className="wiz-copy-code-btn" onClick={handleCopyUserCode} title="复制验证码">
                          {oauthUserCodeCopied ? <Check size={14} /> : <Copy size={14} />}
                        </button>
                      </div>
                    </div>

                    {/* 验证链接 */}
                    <div className="wiz-verification-url">
                      <p className="wiz-uc-label">验证链接：</p>
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
                          <Loader2 size={14} className="spin" />
                          <span>等待您在浏览器中完成授权...</span>
                        </div>
                      )}
                      {oauthTimedOut && (
                        <div className="wiz-poll-row error">
                          <XCircle size={14} />
                          <span>验证码已过期</span>
                          <button className="wiz-link-btn" onClick={handleStartDeviceFlow}>
                            <RotateCw size={12} /> 重新获取
                          </button>
                        </div>
                      )}
                    </div>

                    {/* 重新获取按钮（非过期时也显示） */}
                    {!oauthTimedOut && (
                      <button
                        className="wiz-btn-ghost wiz-retry-btn"
                        onClick={handleStartDeviceFlow}
                        disabled={oauthPreparing}
                      >
                        <RotateCw size={14} /> 重新获取验证码
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

                {/* 方案 B：JSON 文件 */}
                <div className="wiz-import-section">
                  <h4 className="wiz-import-section-title">
                    <FileJson size={15} /> 方案 B — 导入 JSON 文件
                  </h4>
                  <p className="wiz-import-desc">
                    选择一个包含账号数据的 <code>.json</code> 文件批量导入。
                  </p>
                  <button
                    className="wiz-import-btn"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={importing || status === "success"}
                  >
                    <FileJson size={15} />
                    选择 JSON 文件
                  </button>
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept=".json,application/json"
                    style={{ display: "none" }}
                    onChange={e => {
                      const file = e.target.files?.[0];
                      if (file) handleImportJsonFile(file);
                      e.target.value = "";
                    }}
                  />
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
