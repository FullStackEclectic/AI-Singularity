import { invoke } from "@tauri-apps/api/core";
import type { ApiKey, Balance, BalanceSummary, DashboardStats, Platform, ProviderConfig, McpServer } from "../types";

export interface AddKeyRequest {
  name: string;
  platform: Platform;
  secret: string;
  base_url?: string;
  notes?: string;
}

export const api = {
  stats: {
    getDashboard: (): Promise<DashboardStats> => invoke("get_dashboard_stats"),
  },

  keys: {
    list:   (): Promise<ApiKey[]>             => invoke("list_keys"),
    add:    (req: AddKeyRequest): Promise<ApiKey> => invoke("add_key", { request: req }),
    delete: (id: string): Promise<void>       => invoke("delete_key", { id }),
    check:  (id: string)                      => invoke("check_key", { id }),
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
    list:       ()                              => invoke("list_models"),
    byPlatform: (platform: string)              => invoke("get_platform_models", { platform }),
  },

  providers: {
    list:   (): Promise<ProviderConfig[]>       => invoke("get_providers"),
    add:    (provider: any): Promise<void>      => invoke("add_provider", { provider }),
    update: (provider: any): Promise<void>      => invoke("update_provider", { provider }),
    switch: (id: string): Promise<void>         => invoke("switch_provider", { id }),
    delete: (id: string): Promise<void>         => invoke("delete_provider", { id }),
  },

  mcp: {
    list:   (): Promise<McpServer[]>            => invoke("get_mcps"),
    add:    (mcp: any): Promise<void>           => invoke("add_mcp", { mcp }),
    update: (mcp: any): Promise<void>           => invoke("update_mcp", { mcp }),
    toggle: (id: string, isActive: boolean): Promise<void> => invoke("toggle_mcp", { id, isActive }),
    delete: (id: string): Promise<void>         => invoke("delete_mcp", { id }),
  },

  prompts: {
    list:   (): Promise<any[]>                  => invoke("get_prompts"),
    save:   (prompt: any): Promise<void>        => invoke("save_prompt", { prompt }),
    delete: (id: string): Promise<void>         => invoke("delete_prompt", { id }),
    sync:   (id: string, workspaceDir: string): Promise<void> => invoke("sync_prompt", { id, workspaceDir }),
  },

  alerts: {
    get: (): Promise<any[]>                     => invoke("get_alerts"),
  },

  speedtest: {
    run: (): Promise<any[]>                     => invoke("run_speedtest"),
  },
};
