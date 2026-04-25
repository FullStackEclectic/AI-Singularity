import { invoke } from "@tauri-apps/api/core";
import type {
  AccountGroup,
  McpServer,
} from "../../types";
import type {
  CreateFloatingAccountCardRequest,
  CurrentAccountSnapshot,
  FloatingAccountCard,
  GeminiCloudProject,
  IdeStatusActionResult,
  UpdateFloatingAccountCardPatch,
} from "./types";

export const providerCurrent = {
  getAccountId: (platform: string): Promise<string | null> =>
    invoke("get_provider_current_account_id", { platform }),
  listSnapshots: (): Promise<CurrentAccountSnapshot[]> =>
    invoke("list_provider_current_account_snapshots"),
};

export const floatingCards = {
  list: (): Promise<FloatingAccountCard[]> =>
    invoke("list_floating_account_cards"),
  create: (request: CreateFloatingAccountCardRequest): Promise<FloatingAccountCard> =>
    invoke("create_floating_account_card", { request }),
  update: (
    id: string,
    patch: UpdateFloatingAccountCardPatch,
    expectedUpdatedAt?: string | null,
  ): Promise<FloatingAccountCard> =>
    invoke("update_floating_account_card", {
      id,
      patch,
      expectedUpdatedAt: expectedUpdatedAt ?? null,
    }),
  delete: (id: string): Promise<boolean> =>
    invoke("delete_floating_account_card", { id }),
};

export const mcp = {
  list: (): Promise<McpServer[]> => invoke("get_mcps"),
  add: (mcpItem: any): Promise<void> => invoke("add_mcp", { mcp: mcpItem }),
  update: (mcpItem: any): Promise<void> =>
    invoke("update_mcp", { mcp: mcpItem }),
  toggle: (id: string, isActive: boolean): Promise<void> =>
    invoke("toggle_mcp", { id, isActive }),
  delete: (id: string): Promise<void> => invoke("delete_mcp", { id }),
};

export const prompts = {
  list: (): Promise<any[]> => invoke("get_prompts"),
  save: (prompt: any): Promise<void> => invoke("save_prompt", { prompt }),
  delete: (id: string): Promise<void> => invoke("delete_prompt", { id }),
  sync: (id: string, workspaceDir: string): Promise<void> =>
    invoke("sync_prompt", { id, workspaceDir }),
  syncToTool: (id: string): Promise<string[]> =>
    invoke("sync_prompt_to_tool", { id }),
};

export const ideAccounts = {
  list: (): Promise<any[]> => invoke("get_all_ide_accounts"),
  listGroups: (): Promise<AccountGroup[]> => invoke("list_account_groups"),
  createGroup: (name: string): Promise<AccountGroup> =>
    invoke("create_account_group", { name }),
  renameGroup: (id: string, name: string): Promise<AccountGroup> =>
    invoke("rename_account_group", { id, name }),
  deleteGroup: (id: string): Promise<boolean> =>
    invoke("delete_account_group", { id }),
  assignToGroup: (groupId: string, ids: string[]): Promise<AccountGroup> =>
    invoke("assign_ide_accounts_to_group", { groupId, ids }),
  removeFromGroup: (groupId: string, ids: string[]): Promise<AccountGroup> =>
    invoke("remove_ide_accounts_from_group", { groupId, ids }),
  import: (accounts: any[]): Promise<number> =>
    invoke("import_ide_accounts", { accounts }),
  export: (ids: string[]): Promise<string> =>
    invoke("export_ide_accounts", { ids }),
  delete: (id: string): Promise<number> =>
    invoke("delete_ide_account", { id }),
  batchDelete: (ids: string[]): Promise<number> =>
    invoke("batch_delete_ide_accounts", { ids }),
  batchUpdateTags: (ids: string[], tags: string[]): Promise<number> =>
    invoke("batch_update_ide_account_tags", { ids, tags }),
  updateLabel: (id: string, label?: string | null): Promise<void> =>
    invoke("update_ide_account_label", { id, label: label ?? null }),
  refresh: (id: string): Promise<any> => invoke("refresh_ide_account", { id }),
  refreshAllByPlatform: (platform: string): Promise<number> =>
    invoke("refresh_all_ide_accounts_by_platform", { platform }),
  batchRefresh: (ids: string[]): Promise<number> =>
    invoke("batch_refresh_ide_accounts", { ids }),
  listGeminiProjects: (id: string): Promise<GeminiCloudProject[]> =>
    invoke("list_gemini_cloud_projects_for_ide_account", { id }),
  setGeminiProject: (id: string, projectId?: string | null): Promise<any> =>
    invoke("set_gemini_project_for_ide_account", { id, projectId: projectId ?? null }),
  updateCodexApiKey: (id: string, apiKey: string, apiBaseUrl?: string | null): Promise<any> =>
    invoke("update_codex_api_key_credentials_for_ide_account", {
      id,
      apiKey,
      apiBaseUrl: apiBaseUrl ?? null,
    }),
  runStatusAction: (
    id: string,
    action: string,
    retryFailedTimes?: number | null,
  ): Promise<IdeStatusActionResult> =>
    invoke("execute_ide_account_status_action", {
      id,
      action,
      retryFailedTimes: retryFailedTimes ?? null,
    }),
  launchSandbox: (commandStr: string, proxyPort: number): Promise<void> =>
    invoke("launch_tool_sandboxed", { commandStr, proxyPort }),
  forceInject: (accountId: string): Promise<void> =>
    invoke("force_inject_ide", { accountId }),
};

// ===== 账号管理增强：失效检测、批量刷新、自动切号、告警冷却、指纹、Extension 导入 =====

export const accountHealth = {
  listDisabled: (): Promise<any[]> => invoke("list_disabled_ide_accounts"),
  clearDisabled: (id: string): Promise<number> =>
    invoke("clear_ide_account_disabled", { id }),
  markDisabled: (id: string, reason: string): Promise<number> =>
    invoke("mark_ide_account_disabled", { id, reason }),
};

export type RefreshTriggerLabel = "auto" | "manual_batch";
export interface RefreshStats {
  total: number;
  success: number;
  failed: number;
  details: string[];
}
export const accountRefresh = {
  refreshAll: (trigger: RefreshTriggerLabel = "manual_batch"): Promise<RefreshStats> =>
    invoke("refresh_all_ide_accounts", { trigger }),
};

export interface AutoSwitchSettings {
  enabled: boolean;
  threshold: number;
  scopeMode: string;
  selectedGroupIds: string[];
  accountScopeMode: string;
  selectedAccountIds: string[];
  hardSwitchEnabled: boolean;
}
export interface AutoSwitchGroupDefinition {
  id: string;
  name: string;
  models: string[];
}
export interface AutoSwitchOutcome {
  triggered: boolean;
  fromAccountId?: string;
  toAccountId?: string;
  rule?: string;
  reason?: string;
}
export interface AccountSwitchHistoryItem {
  id: string;
  ts: string;
  trigger: string;
  rule?: string | null;
  fromAccountId?: string | null;
  fromEmail?: string | null;
  toAccountId: string;
  toEmail: string;
  reasonJson?: string | null;
}
export const autoSwitch = {
  getSettings: (): Promise<AutoSwitchSettings> =>
    invoke("get_auto_switch_settings"),
  setSettings: (settings: AutoSwitchSettings): Promise<AutoSwitchSettings> =>
    invoke("set_auto_switch_settings", { settings }),
  listGroups: (): Promise<AutoSwitchGroupDefinition[]> =>
    invoke("list_auto_switch_groups"),
  runNow: (): Promise<AutoSwitchOutcome> => invoke("run_auto_switch_now"),
  listHistory: (limit = 50): Promise<AccountSwitchHistoryItem[]> =>
    invoke("list_account_switch_history", { limit }),
};

export interface QuotaAlertSettings {
  enabled: boolean;
  threshold: number;
  cooldownSeconds: number;
}
export interface QuotaAlertPayload {
  accountId: string;
  email: string;
  originPlatform: string;
  threshold: number;
  lowestPercentage: number;
  lowModels: string[];
  triggeredAt: number;
}
export const quotaAlert = {
  getSettings: (): Promise<QuotaAlertSettings> =>
    invoke("get_quota_alert_settings"),
  setSettings: (settings: QuotaAlertSettings): Promise<QuotaAlertSettings> =>
    invoke("set_quota_alert_settings", { settings }),
  preview: (): Promise<QuotaAlertPayload[]> => invoke("preview_quota_alerts"),
};

export interface DeviceFingerprintRecord {
  id: string;
  name: string;
  machineId: string;
  macMachineId: string;
  devDeviceId: string;
  sqmId: string;
  serviceMachineId?: string | null;
  createdAt: string;
}
export const deviceFingerprints = {
  list: (): Promise<DeviceFingerprintRecord[]> =>
    invoke("list_device_fingerprints"),
  create: (
    name: string,
    seed?: DeviceFingerprintRecord | null,
  ): Promise<DeviceFingerprintRecord> =>
    invoke("create_device_fingerprint", { name, seed: seed ?? null }),
  rename: (id: string, name: string): Promise<number> =>
    invoke("rename_device_fingerprint", { id, name }),
  delete: (id: string): Promise<number> =>
    invoke("delete_device_fingerprint", { id }),
  applyToAccount: (
    accountId: string,
    fingerprintId?: string | null,
  ): Promise<number> =>
    invoke("apply_device_fingerprint_to_account", {
      accountId,
      fingerprintId: fingerprintId ?? null,
    }),
};

export interface ExtensionScanResult {
  source: string;
  extensionId: string;
  email: string;
  projectId?: string | null;
  hasRefreshToken: boolean;
}
export interface ExtensionImportStats {
  scanned: number;
  imported: number;
  skipped: number;
  failed: number;
  details: string[];
}
export const extensionImport = {
  scan: (): Promise<ExtensionScanResult[]> =>
    invoke("scan_extension_credentials"),
  importAll: (): Promise<ExtensionImportStats> =>
    invoke("import_from_extension"),
};

export const userTokens = {
  list: (): Promise<any[]> => invoke("get_all_user_tokens"),
  create: (req: any): Promise<any> => invoke("create_user_token", { req }),
  update: (req: any): Promise<void> => invoke("update_user_token", { req }),
  delete: (id: string): Promise<void> => invoke("delete_user_token", { id }),
};

export const analytics = {
  getDashboardMetrics: (days: number): Promise<any> =>
    invoke("get_dashboard_metrics", { days }),
};
