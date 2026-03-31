import { invoke } from "@tauri-apps/api/core";
import type { ApiKey, KeyStatus, Platform } from "../types";

export interface AddKeyRequest {
  name: string;
  platform: Platform;
  secret: string;
  base_url?: string;
  notes?: string;
}

export const api = {
  keys: {
    list: (): Promise<ApiKey[]> => invoke("list_keys"),
    add: (req: AddKeyRequest): Promise<ApiKey> => invoke("add_key", { request: req }),
    delete: (id: string): Promise<void> => invoke("delete_key", { id }),
    check: (id: string): Promise<KeyStatus> => invoke("check_key", { id }),
  },
  balance: {
    listAll: () => invoke("get_all_balances"),
  },
  models: {
    list: () => invoke("list_models"),
    byPlatform: (platform: string) => invoke("get_platform_models", { platform }),
  },
};
