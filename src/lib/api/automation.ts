import { invoke } from "@tauri-apps/api/core";
import type {
  GeminiInstanceLaunchInfo,
  GeminiInstanceRecord,
  OAuthEnvStatusItem,
  WakeupHistoryItem,
  WakeupState,
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
