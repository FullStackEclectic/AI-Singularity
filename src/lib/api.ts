import { invoke } from "@tauri-apps/api/core";
import type { ApiKey, Balance, BalanceSummary, DashboardStats, Platform, ProviderConfig, McpServer, EnvConflict, TokenUsageStat, Model } from "../types";

export interface AddKeyRequest {
  name: string;
  platform: Platform;
  secret: string;
  base_url?: string;
  notes?: string;
}

export interface SkillStorageInfo {
  primary_path: string;
  legacy_path: string;
  legacy_exists: boolean;
}

export interface OAuthEnvStatusItem {
  provider: string;
  env_name: string;
  configured: boolean;
}

export interface GeminiCloudProject {
  project_id: string;
  project_name?: string | null;
}

export interface GeminiInstanceRecord {
  id: string;
  name: string;
  user_data_dir: string;
  extra_args?: string;
  bind_account_id?: string | null;
  project_id?: string | null;
  last_launched_at?: string | null;
  initialized: boolean;
  is_default?: boolean;
}

export interface GeminiInstanceLaunchInfo {
  instance_id: string;
  user_data_dir: string;
  launch_command: string;
}

export interface DesktopLogFile {
  name: string;
  path: string;
  size: number;
  modified_at?: string | null;
  kind: string;
}

export interface DesktopLogReadResult {
  name: string;
  path: string;
  total_lines: number;
  matched_lines: number;
  content: string;
}

export interface UpdateSettings {
  auto_check: boolean;
  auto_install: boolean;
  last_check_at?: string | null;
}

export interface UpdateRuntimeInfo {
  current_version: string;
  platform: string;
  updater_endpoints: string[];
  updater_pubkey_configured: boolean;
  can_auto_install: boolean;
  linux_install_kind?: string | null;
  linux_manual_hint?: string | null;
  warning?: string | null;
}

export interface LinuxReleaseAssetInfo {
  name: string;
  kind: string;
  url: string;
  size?: number | null;
  content_type?: string | null;
  preferred: boolean;
}

export interface LinuxReleaseInfo {
  version: string;
  published_at?: string | null;
  body?: string | null;
  assets: LinuxReleaseAssetInfo[];
}

export interface LinuxInstallResult {
  downloaded_path: string;
  action: string;
  message: string;
}

export interface WebSocketStatus {
  running: boolean;
  port?: number | null;
  client_count: number;
}

export interface AnnouncementAction {
  type: string;
  target: string;
  label: string;
}

export interface Announcement {
  id: string;
  type: string;
  priority: number;
  title: string;
  summary: string;
  content: string;
  action?: AnnouncementAction | null;
  target_versions: string;
  target_languages?: string[];
  show_once?: boolean;
  popup: boolean;
  created_at: string;
  expires_at?: string | null;
}

export interface AnnouncementState {
  announcements: Announcement[];
  unread_ids: string[];
  popup_announcement?: Announcement | null;
}

export interface WakeupTask {
  id: string;
  name: string;
  enabled: boolean;
  account_id: string;
  command_template: string;
  model: string;
  prompt: string;
  cron: string;
  notes?: string | null;
  timeout_seconds: number;
  created_at: string;
  updated_at: string;
  last_run_at?: string | null;
  last_status?: string | null;
  last_message?: string | null;
}

export interface WakeupState {
  enabled: boolean;
  tasks: WakeupTask[];
}

export interface WakeupHistoryItem {
  id: string;
  task_id?: string | null;
  task_name: string;
  account_id: string;
  model: string;
  status: string;
  message?: string | null;
  created_at: string;
}

export interface WakeupVerificationBatchItem {
  account_id: string;
  email: string;
  status: string;
  message: string;
}

export interface WakeupVerificationBatchResult {
  executed_count: number;
  success_count: number;
  failed_count: number;
  items: WakeupVerificationBatchItem[];
}

export const api = {
  stats: {
    getDashboard: (): Promise<DashboardStats>        => invoke("get_dashboard_stats"),
    getTokenUsage: (): Promise<{ by_app: TokenUsageStat[]; by_model: TokenUsageStat[] }> =>
      invoke("get_token_usage_stats"),
  },

  env: {
    checkConflicts: (appName: string): Promise<EnvConflict[]> => invoke("check_system_env_conflicts", { appName }),
  },

  proxy: {
    syncEngineConfig: (config: any): Promise<void> => invoke("sync_proxy_engine_config", { config }),
  },

  security: {
    getAccessLogs: (limit?: number): Promise<any[]> => invoke("get_ip_access_logs", { limit }),
    clearAccessLogs: (): Promise<void> => invoke("clear_ip_access_logs"),
    getRules: (): Promise<any[]> => invoke("get_ip_rules"),
    addRule: (ipCidr: string, ruleType: string, notes?: string): Promise<void> => invoke("add_ip_rule", { ipCidr, ruleType, notes }),
    deleteRule: (id: string): Promise<void> => invoke("delete_ip_rule", { id }),
    toggleRule: (id: string, active: boolean): Promise<void> => invoke("toggle_ip_rule", { id, active }),
  },

  keys: {
    list:           (): Promise<ApiKey[]>              => invoke("list_keys"),
    add:            (req: AddKeyRequest): Promise<ApiKey> => invoke("add_key", { request: req }),
    delete:         (id: string): Promise<void>        => invoke("delete_key", { id }),
    check:          (id: string)                       => invoke("check_key", { id }),
    updatePriority: (id: string, priority: number): Promise<void> =>
      invoke("update_key", { request: { id, priority } }),
  },

  balance: {
    listAll:         (): Promise<Balance[]>          => invoke("get_all_balances"),
    refreshOne:      (key_id: string): Promise<Balance> => invoke("get_platform_balance", { key_id }),
    refreshAll:      (): Promise<Balance[]>          => invoke("refresh_all_balances"),
    summaries:       (): Promise<BalanceSummary[]>   => invoke("get_balance_summaries"),
    refreshProviders:(): Promise<any[]>              => invoke("refresh_provider_balances"),
    refreshProvider: (providerId: string): Promise<any> => invoke("refresh_provider_balance", { providerId }),
    history:         (providerId: string, limit?: number): Promise<any[]> =>
      invoke("get_balance_history", { providerId, limit: limit ?? 30 }),
  },

  models: {
    list:       (): Promise<Model[]>            => invoke("list_models"),
    byPlatform: (platform: string): Promise<Model[]> => invoke("get_platform_models", { platform }),
  },

  providers: {
    list:   (): Promise<ProviderConfig[]>       => invoke("get_providers"),
    add:    (provider: any): Promise<void>      => invoke("add_provider", { provider }),
    update: (provider: any): Promise<void>      => invoke("update_provider", { provider }),
    switch: (id: string): Promise<void>         => invoke("switch_provider", { id }),
    delete: (id: string): Promise<void>         => invoke("delete_provider", { id }),
    updateOrder: (ids: string[]): Promise<void> => invoke("update_providers_order", { ids }),
    streamCheck: (id: string): Promise<any>     => invoke("stream_check_provider", { providerId: id }),
    fetchModels: (request: {
      platform: Platform;
      base_url?: string;
      api_key_value?: string;
      api_key_id?: string;
    }): Promise<string[]> => invoke("fetch_provider_models", { request }),
  },

  providerCurrent: {
    getAccountId: (platform: string): Promise<string | null> =>
      invoke("get_provider_current_account_id", { platform }),
  },

  mcp: {
    list:   (): Promise<McpServer[]>            => invoke("get_mcps"),
    add:    (mcp: any): Promise<void>           => invoke("add_mcp", { mcp }),
    update: (mcp: any): Promise<void>           => invoke("update_mcp", { mcp }),
    toggle: (id: string, isActive: boolean): Promise<void> => invoke("toggle_mcp", { id, isActive }),
    delete: (id: string): Promise<void>         => invoke("delete_mcp", { id }),
  },

  prompts: {
    list:       (): Promise<any[]>                  => invoke("get_prompts"),
    save:       (prompt: any): Promise<void>        => invoke("save_prompt", { prompt }),
    delete:     (id: string): Promise<void>         => invoke("delete_prompt", { id }),
    sync:       (id: string, workspaceDir: string): Promise<void> => invoke("sync_prompt", { id, workspaceDir }),
    syncToTool: (id: string): Promise<string[]>     => invoke("sync_prompt_to_tool", { id }),
  },

  alerts: {
    get: (): Promise<any[]>                     => invoke("get_alerts"),
  },

  speedtest: {
    run: (): Promise<any[]>                     => invoke("run_speedtest"),
  },

  ideAccounts: {
    list:   (): Promise<any[]>                  => invoke("get_all_ide_accounts"),
    import: (accounts: any[]): Promise<number>  => invoke("import_ide_accounts", { accounts }),
    export: (ids: string[]): Promise<string>    => invoke("export_ide_accounts", { ids }),
    delete: (id: string): Promise<number>       => invoke("delete_ide_account", { id }),
    updateLabel: (id: string, label?: string | null): Promise<void> =>
      invoke("update_ide_account_label", { id, label: label ?? null }),
    refresh: (id: string): Promise<any>         => invoke("refresh_ide_account", { id }),
    refreshAllByPlatform: (platform: string): Promise<number> =>
      invoke("refresh_all_ide_accounts_by_platform", { platform }),
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
    launchSandbox: (commandStr: string, proxyPort: number): Promise<void> => 
      invoke("launch_tool_sandboxed", { commandStr, proxyPort }),
    forceInject: (accountId: string): Promise<void> =>
      invoke("force_inject_ide", { accountId }),
  },

  tools: {
    checkStatus: (toolId: string): Promise<any> => invoke("check_tool_status", { toolId }),
    deploy: (toolId: string): Promise<void> => invoke("deploy_tool", { toolId }),
  },

  userTokens: {
    list:   (): Promise<any[]>                  => invoke("get_all_user_tokens"),
    create: (req: any): Promise<any>            => invoke("create_user_token", { req }),
    update: (req: any): Promise<void>           => invoke("update_user_token", { req }),
    delete: (id: string): Promise<void>         => invoke("delete_user_token", { id }),
  },

  analytics: {
    getDashboardMetrics: (days: number): Promise<any> => invoke("get_dashboard_metrics", { days }),
  },

  logs: {
    list: (): Promise<DesktopLogFile[]> => invoke("list_desktop_logs"),
    read: (name: string, lines?: number, query?: string): Promise<DesktopLogReadResult> =>
      invoke("read_desktop_log", { name, lines: lines ?? 500, query: query?.trim() || null }),
    export: (name: string, destination: string): Promise<void> =>
      invoke("export_desktop_log", { name, destination }),
  },

  update: {
    getSettings: (): Promise<UpdateSettings> => invoke("get_update_settings"),
    saveSettings: (settings: UpdateSettings): Promise<void> => invoke("save_update_settings", { settings }),
    markCheckedNow: (): Promise<UpdateSettings> => invoke("update_last_check_time"),
    getRuntimeInfo: (): Promise<UpdateRuntimeInfo> => invoke("get_update_runtime_info"),
    getLinuxReleaseInfo: (): Promise<LinuxReleaseInfo> => invoke("get_linux_update_release_info"),
    openAssetUrl: (url: string): Promise<void> => invoke("open_update_asset_url", { url }),
    installLinuxAsset: (request: {
      url: string;
      kind: string;
      version?: string | null;
    }): Promise<LinuxInstallResult> =>
      invoke("install_linux_update_asset", {
        url: request.url,
        kind: request.kind,
        version: request.version ?? null,
      }),
  },

  websocket: {
    getStatus: (): Promise<WebSocketStatus> => invoke("get_websocket_status"),
  },

  announcements: {
    getState: (locale?: string): Promise<AnnouncementState> =>
      invoke("announcement_get_state", { locale: locale ?? null }),
    markRead: (id: string): Promise<void> => invoke("announcement_mark_as_read", { id }),
    markAllRead: (locale?: string): Promise<void> =>
      invoke("announcement_mark_all_as_read", { locale: locale ?? null }),
    refresh: (locale?: string): Promise<AnnouncementState> =>
      invoke("announcement_force_refresh", { locale: locale ?? null }),
  },

  wakeup: {
    getState: (): Promise<WakeupState> => invoke("wakeup_get_state"),
    saveState: (state: WakeupState): Promise<WakeupState> => invoke("wakeup_save_state", { state }),
    runTaskNow: (taskId: string): Promise<WakeupState> => invoke("wakeup_run_task_now", { taskId }),
    loadHistory: (): Promise<WakeupHistoryItem[]> => invoke("wakeup_load_history"),
    addHistory: (items: WakeupHistoryItem[]): Promise<WakeupHistoryItem[]> =>
      invoke("wakeup_add_history", { items }),
    clearHistory: (): Promise<void> => invoke("wakeup_clear_history"),
    runVerificationBatch: (request: {
      accountIds: string[];
      model: string;
      prompt: string;
      commandTemplate: string;
      timeoutSeconds?: number;
    }): Promise<WakeupVerificationBatchResult> =>
      invoke("wakeup_run_verification_batch", {
        accountIds: request.accountIds,
        model: request.model,
        prompt: request.prompt,
        commandTemplate: request.commandTemplate,
        timeoutSeconds: request.timeoutSeconds ?? 120,
      }),
  },

  oauth: {
    startFlow: (provider: string): Promise<string> => invoke("start_oauth_flow", { provider }),
    getEnvStatus: (): Promise<OAuthEnvStatusItem[]> => invoke("get_oauth_env_status"),
  },

  geminiInstances: {
    list: (): Promise<GeminiInstanceRecord[]> => invoke("list_gemini_instances"),
    getDefault: (): Promise<GeminiInstanceRecord> => invoke("get_default_gemini_instance"),
    add: (name: string, userDataDir: string): Promise<GeminiInstanceRecord> =>
      invoke("add_gemini_instance", { name, userDataDir }),
    delete: (id: string): Promise<void> => invoke("delete_gemini_instance", { id }),
    update: (
      id: string,
      extraArgs?: string | null,
      bindAccountId?: string | null,
      projectId?: string | null,
    ): Promise<GeminiInstanceRecord> =>
      invoke("update_gemini_instance_settings", {
        id,
        extraArgs: extraArgs ?? null,
        bindAccountId: bindAccountId ?? null,
        projectId: projectId ?? null,
      }),
    getLaunchCommand: (id: string): Promise<GeminiInstanceLaunchInfo> =>
      invoke("get_gemini_instance_launch_command", { id }),
    launch: (id: string): Promise<string> => invoke("launch_gemini_instance", { id }),
  },

  webdav: {
    testConnection: (config: any): Promise<void> => invoke("webdav_test_connection", { config }),
    push: (config: any): Promise<void> => invoke("webdav_push", { config }),
    pull: (config: any): Promise<void> => invoke("webdav_pull", { config }),
  },

  skills: {
    getStorageInfo: (): Promise<SkillStorageInfo> => invoke("get_skill_storage_info"),
  },
};
