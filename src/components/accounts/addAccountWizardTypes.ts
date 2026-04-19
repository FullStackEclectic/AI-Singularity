import type { ReactNode } from "react";

export type AccountMode = "api_key" | "sandbox";
export type SandboxTab = "oauth" | "token" | "import";
export type Status = "idle" | "loading" | "success" | "error";
export type IdeOrigin =
  | "antigravity"
  | "claude_code"
  | "cursor"
  | "windsurf"
  | "github_copilot"
  | "claude_desktop"
  | "zed"
  | "vscode"
  | "opencode"
  | "codex"
  | "kiro"
  | "gemini"
  | "codebuddy"
  | "codebuddy_cn"
  | "workbuddy"
  | "trae"
  | "qoder"
  | "generic_ide";

export interface DeviceFlowStart {
  login_id: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval_seconds: number;
}

export interface ImportSummaryItem {
  label: string;
  origin_platform: string;
  source_path: string;
  reason?: string;
}

export interface ImportSummary {
  ok: number;
  fail: number;
  successes: ImportSummaryItem[];
  failures: ImportSummaryItem[];
}

export interface ScannedIdeAccount {
  email: string;
  refresh_token: string | null;
  access_token: string | null;
  origin_platform: string;
  source_path: string;
  meta_json?: string | null;
  label?: string | null;
}

export interface FileImportFailure {
  source_path: string;
  reason: string;
}

export interface FileImportScanResult {
  accounts: ScannedIdeAccount[];
  failures: FileImportFailure[];
}

export interface LocalImportOptionView {
  title: string;
  description: ReactNode;
  buttonLabel: string;
}

export interface LocalImportOption extends LocalImportOptionView {
  loadingMessage: string;
  command: string;
  fallbackOrigin: IdeOrigin;
  successMessage: string;
  emptyMessage: string;
}

export interface ChannelOption {
  value: string;
  label: string;
  desc: string;
}
