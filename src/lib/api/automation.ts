import { invoke } from "@tauri-apps/api/core";
import type {
  GeminiInstanceLaunchInfo,
  GeminiInstanceRecord,
  OAuthEnvStatusItem,
  WakeupHistoryItem,
  WakeupRunHistoryRow,
  WakeupRunsPage,
  WakeupRuntimeStatus,
  WakeupState,
  WakeupSummary24h,
  WakeupVerificationBatchResult,
} from "./types";

export const wakeup = {
  getState: (): Promise<WakeupState> => invoke("wakeup_get_state"),
  saveState: (state: WakeupState): Promise<WakeupState> =>
    invoke("wakeup_save_state", { state }),
  runTaskNow: (taskId: string): Promise<WakeupState> =>
    invoke("wakeup_run_task_now", { taskId }),
  loadHistory: (): Promise<WakeupHistoryItem[]> =>
    invoke("wakeup_load_history"),
  addHistory: (items: WakeupHistoryItem[]): Promise<WakeupHistoryItem[]> =>
    invoke("wakeup_add_history", { items }),
  clearHistory: (): Promise<void> => invoke("wakeup_clear_history"),
  runVerificationBatch: (request: {
    accountIds: string[];
    model: string;
    prompt: string;
    commandTemplate: string;
    timeoutSeconds?: number;
    retryFailedTimes?: number;
    runId?: string;
  }): Promise<WakeupVerificationBatchResult> =>
    invoke("wakeup_run_verification_batch", {
      accountIds: request.accountIds,
      model: request.model,
      prompt: request.prompt,
      commandTemplate: request.commandTemplate,
      timeoutSeconds: request.timeoutSeconds ?? 120,
      retryFailedTimes: request.retryFailedTimes ?? 1,
      runId: request.runId ?? null,
    }),
  cancelVerificationRun: (runId: string): Promise<boolean> =>
    invoke("wakeup_cancel_verification_run", { runId }),
  listRuns: (
    kind?: string,
    limit = 50,
    offset = 0,
  ): Promise<WakeupRunsPage> =>
    invoke("wakeup_list_runs", {
      kind: kind ?? null,
      limit,
      offset,
    }),
  getRunItems: (runId: string): Promise<WakeupRunHistoryRow[]> =>
    invoke("wakeup_get_run_items", { runId }),
  getRuntimeStatus: (): Promise<WakeupRuntimeStatus> =>
    invoke("wakeup_get_runtime_status"),
  getSummary24h: (): Promise<WakeupSummary24h> =>
    invoke("wakeup_get_summary_24h"),
};

export const oauth = {
  startFlow: (provider: string): Promise<string> =>
    invoke("start_oauth_flow", { provider }),
  getEnvStatus: (): Promise<OAuthEnvStatusItem[]> =>
    invoke("get_oauth_env_status"),
};

export const geminiInstances = {
  list: (): Promise<GeminiInstanceRecord[]> => invoke("list_gemini_instances"),
  getDefault: (): Promise<GeminiInstanceRecord> =>
    invoke("get_default_gemini_instance"),
  add: (name: string, userDataDir: string): Promise<GeminiInstanceRecord> =>
    invoke("add_gemini_instance", { name, userDataDir }),
  delete: (id: string): Promise<void> =>
    invoke("delete_gemini_instance", { id }),
  update: (
    id: string,
    extraArgs?: string | null,
    bindAccountId?: string | null,
    projectId?: string | null,
    followLocalAccount?: boolean | null,
  ): Promise<GeminiInstanceRecord> =>
    invoke("update_gemini_instance_settings", {
      id,
      extraArgs: extraArgs ?? null,
      bindAccountId: bindAccountId ?? null,
      projectId: projectId ?? null,
      followLocalAccount: followLocalAccount ?? null,
    }),
  getLaunchCommand: (id: string): Promise<GeminiInstanceLaunchInfo> =>
    invoke("get_gemini_instance_launch_command", { id }),
  launch: (id: string): Promise<string> =>
    invoke("launch_gemini_instance", { id }),
};

export const webdav = {
  testConnection: (config: any): Promise<void> =>
    invoke("webdav_test_connection", { config }),
  push: (config: any): Promise<void> => invoke("webdav_push", { config }),
  pull: (config: any): Promise<void> => invoke("webdav_pull", { config }),
};

export type CodexQuotaCacheStats = {
  total: number;
  valid: number;
  hitTotal: number;
  lastWrittenAt: string | null;
};

export type TokenHealthOverview = {
  expiringWithin1h: number;
  alreadyExpired: number;
  lastKeeperTick: string | null;
  lastKeeperRescues: number;
};

export const codexQuota = {
  getCacheStats: (): Promise<CodexQuotaCacheStats> =>
    invoke("codex_get_quota_cache_stats"),
  clearCache: (accountId?: string): Promise<number> =>
    invoke("codex_clear_quota_cache", { accountId: accountId ?? null }),
};

export const accountHealth = {
  getTokenOverview: (): Promise<TokenHealthOverview> =>
    invoke("get_token_health_overview"),
};
