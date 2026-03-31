import { invoke } from "@tauri-apps/api/core";
import type { ApiKey, Balance, DashboardStats, Platform } from "../types";

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
    list: (): Promise<ApiKey[]> => invoke("list_keys"),
    add: (req: AddKeyRequest): Promise<ApiKey> => invoke("add_key", { request: req }),
    delete: (id: string): Promise<void> => invoke("delete_key", { id }),
    check: (id: string) => invoke("check_key", { id }),
  },
  balance: {
    listAll: (): Promise<Balance[]> => invoke("get_all_balances"),
  },
  models: {
    list: () => invoke("list_models"),
    byPlatform: (platform: string) => invoke("get_platform_models", { platform }),
  },
};
