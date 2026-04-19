import type { LocalImportOption } from "./addAccountWizardTypes";

export const IDE_ORIGINS = [
  { value: "antigravity", label: "Antigravity", desc: "Claude OAuth 账号池（主渠道）" },
  { value: "claude_code", label: "Claude Code", desc: "Anthropic CLI 终端授权" },
  { value: "cursor", label: "Cursor", desc: "AI 编辑器 OAuth 账号" },
  { value: "windsurf", label: "Windsurf", desc: "Codeium 沙盒账号" },
  { value: "github_copilot", label: "GitHub Copilot", desc: "设备授权令牌" },
  { value: "claude_desktop", label: "Claude Desktop", desc: "桌面客户端凭证" },
  { value: "zed", label: "Zed", desc: "原生编辑器账号" },
  { value: "vscode", label: "VS Code", desc: "全球化存储库账号" },
  { value: "opencode", label: "OpenCode", desc: "沙盒授权账号" },
  { value: "codex", label: "OpenAI Codex", desc: "API 授权令牌" },
  { value: "kiro", label: "Kiro", desc: "第三方 IDE 账号" },
  { value: "gemini", label: "Gemini CLI", desc: "Google 授权账号" },
  { value: "codebuddy", label: "CodeBuddy", desc: "腾讯云 AI 助手" },
  { value: "codebuddy_cn", label: "CodeBuddy CN", desc: "腾讯云 AI 助手国区版" },
  { value: "workbuddy", label: "WorkBuddy", desc: "腾讯云国际版 AI 助手" },
  { value: "trae", label: "Trae", desc: "字节跳动 AI 编辑器" },
  { value: "qoder", label: "Qoder", desc: "Qoding 沙盒账号" },
  { value: "generic_ide", label: "其它 / 通用", desc: "自定义 OAuth 账号" },
] as const;

export const LOCAL_IMPORT_OPTIONS: Partial<Record<string, LocalImportOption>> = {
  gemini: {
    title: "方案 B — 导入本地 Gemini 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>~/.gemini/oauth_creds.json</code> 与{" "}
        <code>google_accounts.json</code>。
      </>
    ),
    buttonLabel: "导入本地 Gemini 登录",
    loadingMessage: "正在读取本地 Gemini 登录信息...",
    command: "import_gemini_from_local",
    fallbackOrigin: "gemini",
    successMessage: "本地 Gemini 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Gemini 账号",
  },
  codex: {
    title: "方案 B — 导入本地 Codex 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>~/.codex/auth.json</code>。
      </>
    ),
    buttonLabel: "导入本地 Codex 登录",
    loadingMessage: "正在读取本地 Codex 登录信息...",
    command: "import_codex_from_local",
    fallbackOrigin: "codex",
    successMessage: "本地 Codex 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Codex 账号",
  },
  kiro: {
    title: "方案 B — 导入本地 Kiro 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>~/.aws/sso/cache/kiro-auth-token.json</code>{" "}
        与本地 <code>profile.json</code>。
      </>
    ),
    buttonLabel: "导入本地 Kiro 登录",
    loadingMessage: "正在读取本地 Kiro 登录...",
    command: "import_kiro_from_local",
    fallbackOrigin: "kiro",
    successMessage: "本地 Kiro 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Kiro 账号",
  },
  cursor: {
    title: "方案 B — 导入本地 Cursor 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>Cursor/User/globalStorage/state.vscdb</code>{" "}
        中的登录态。
      </>
    ),
    buttonLabel: "导入本地 Cursor 登录",
    loadingMessage: "正在读取本地 Cursor 登录...",
    command: "import_cursor_from_local",
    fallbackOrigin: "cursor",
    successMessage: "本地 Cursor 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Cursor 账号",
  },
  windsurf: {
    title: "方案 B — 导入本地 Windsurf 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>Windsurf/User/globalStorage/state.vscdb</code>{" "}
        中的登录态。
      </>
    ),
    buttonLabel: "导入本地 Windsurf 登录",
    loadingMessage: "正在读取本地 Windsurf 登录...",
    command: "import_windsurf_from_local",
    fallbackOrigin: "windsurf",
    successMessage: "本地 Windsurf 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Windsurf 账号",
  },
  codebuddy: {
    title: "方案 B — 导入本地 CodeBuddy 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>CodeBuddy/User/globalStorage/state.vscdb</code>{" "}
        中的登录态。
      </>
    ),
    buttonLabel: "导入本地 CodeBuddy 登录",
    loadingMessage: "正在读取本地 CodeBuddy 登录...",
    command: "import_codebuddy_from_local",
    fallbackOrigin: "codebuddy",
    successMessage: "本地 CodeBuddy 账号导入完成",
    emptyMessage: "未读取到可导入的本地 CodeBuddy 账号",
  },
  codebuddy_cn: {
    title: "方案 B — 导入本地 CodeBuddy CN 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>CodeBuddy CN/User/globalStorage/state.vscdb</code>{" "}
        中的登录态。
      </>
    ),
    buttonLabel: "导入本地 CodeBuddy CN 登录",
    loadingMessage: "正在读取本地 CodeBuddy CN 登录...",
    command: "import_codebuddy_cn_from_local",
    fallbackOrigin: "codebuddy_cn",
    successMessage: "本地 CodeBuddy CN 账号导入完成",
    emptyMessage: "未读取到可导入的本地 CodeBuddy CN 账号",
  },
  workbuddy: {
    title: "方案 B — 导入本地 WorkBuddy 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>WorkBuddy/User/globalStorage/state.vscdb</code>{" "}
        中的登录态。
      </>
    ),
    buttonLabel: "导入本地 WorkBuddy 登录",
    loadingMessage: "正在读取本地 WorkBuddy 登录...",
    command: "import_workbuddy_from_local",
    fallbackOrigin: "workbuddy",
    successMessage: "本地 WorkBuddy 账号导入完成",
    emptyMessage: "未读取到可导入的本地 WorkBuddy 账号",
  },
  zed: {
    title: "方案 B — 导入本地 Zed 登录",
    description: (
      <>
        直接读取当前系统中的 <code>Zed Keychain</code> 登录信息。该能力当前仅在
        macOS 上可用。
      </>
    ),
    buttonLabel: "导入本地 Zed 登录",
    loadingMessage: "正在读取本地 Zed 登录...",
    command: "import_zed_from_local",
    fallbackOrigin: "zed",
    successMessage: "本地 Zed 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Zed 账号",
  },
  qoder: {
    title: "方案 B — 导入本地 Qoder 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>Qoder/User/globalStorage/state.vscdb</code>{" "}
        中的本地登录信息。
      </>
    ),
    buttonLabel: "导入本地 Qoder 登录",
    loadingMessage: "正在读取本地 Qoder 登录...",
    command: "import_qoder_from_local",
    fallbackOrigin: "qoder",
    successMessage: "本地 Qoder 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Qoder 账号",
  },
  trae: {
    title: "方案 B — 导入本地 Trae 登录",
    description: (
      <>
        直接读取当前用户目录下的 <code>Trae/User/globalStorage/storage.json</code>。
      </>
    ),
    buttonLabel: "导入本地 Trae 登录",
    loadingMessage: "正在读取本地 Trae 登录...",
    command: "import_trae_from_local",
    fallbackOrigin: "trae",
    successMessage: "本地 Trae 账号导入完成",
    emptyMessage: "未读取到可导入的本地 Trae 账号",
  },
};

const DEVICE_FLOW_PROVIDERS: readonly string[] = ["github_copilot"];
const IMPORT_ONLY_PROVIDERS: readonly string[] = [
  "claude_code",
  "claude_desktop",
  "vscode",
  "opencode",
  "generic_ide",
];

export const isDeviceFlow = (provider: string) => DEVICE_FLOW_PROVIDERS.includes(provider);
export const isImportOnly = (provider: string) => IMPORT_ONLY_PROVIDERS.includes(provider);
export const isBrowserOAuth = (provider: string) =>
  !isDeviceFlow(provider) && !isImportOnly(provider);
