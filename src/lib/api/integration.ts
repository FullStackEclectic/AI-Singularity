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
