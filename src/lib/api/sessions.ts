import { invoke } from "@tauri-apps/api/core";

export const sessions = {
  list: (instanceId?: string | null): Promise<any[]> =>
    invoke("list_sessions", { instanceId: instanceId ?? null }),
  getDetails: (filepath: string): Promise<any[]> =>
    invoke("get_session_details", { filepath }),
  scanZombies: (): Promise<any[]> =>
    invoke("scan_zombies"),
  launchTerminal: (cwd: string, toolType?: string | null): Promise<void> =>
    invoke("launch_session_terminal", { cwd, toolType: toolType ?? null }),
  moveToTrash: (filepaths: string[]): Promise<number> =>
    invoke("move_sessions_to_trash", { filepaths }),
  repairCodexIndex: (instanceId?: string | null): Promise<number> =>
    invoke("repair_codex_session_index", { instanceId: instanceId ?? null }),
  syncCodexThreads: (): Promise<number> =>
    invoke("sync_codex_threads_across_instances"),
  // Codex instances
  listCodexInstances: (): Promise<any[]> =>
    invoke("list_codex_instances"),
  getDefaultCodexInstance: (): Promise<any> =>
    invoke("get_default_codex_instance"),
  addCodexInstance: (name: string, userDataDir: string): Promise<any> =>
    invoke("add_codex_instance", { name, userDataDir }),
  deleteCodexInstance: (id: string): Promise<void> =>
    invoke("delete_codex_instance", { id }),
  updateCodexInstanceSettings: (
    id: string,
    extraArgs?: string | null,
    bindAccountId?: string | null,
    followLocalAccount?: boolean | null,
  ): Promise<any> =>
    invoke("update_codex_instance_settings", {
      id,
      extraArgs: extraArgs ?? null,
      bindAccountId: bindAccountId ?? null,
      followLocalAccount: followLocalAccount ?? null,
    }),
  startCodexInstance: (id: string): Promise<void> =>
    invoke("start_codex_instance", { id }),
  stopCodexInstance: (id: string): Promise<void> =>
    invoke("stop_codex_instance", { id }),
  openCodexInstanceWindow: (id: string): Promise<void> =>
    invoke("open_codex_instance_window", { id }),
  closeAllCodexInstances: (): Promise<void> =>
    invoke("close_all_codex_instances"),
  syncCodexInstanceSharedResources: (id: string): Promise<void> =>
    invoke("sync_codex_instance_shared_resources", { id }),
};
