import { invoke } from "@tauri-apps/api/core";
import type {
  ApiKey,
  Balance,
  BalanceSummary,
  Model,
  Platform,
} from "../../types";
import type {
  AddKeyRequest,
  FetchRemoteModelPricingResponse,
} from "./types";

export const keys = {
  list: (): Promise<ApiKey[]> => invoke("list_keys"),
  add: (req: AddKeyRequest): Promise<ApiKey> =>
    invoke("add_key", { request: req }),
  delete: (id: string): Promise<void> => invoke("delete_key", { id }),
  check: (id: string) => invoke("check_key", { id }),
  updatePriority: (id: string, priority: number): Promise<void> =>
    invoke("update_key", { request: { id, priority } }),
};

export const balance = {
  listAll: (): Promise<Balance[]> => invoke("get_all_balances"),
  refreshOne: (key_id: string): Promise<Balance> =>
    invoke("get_platform_balance", { key_id }),
  refreshAll: (): Promise<Balance[]> => invoke("refresh_all_balances"),
  summaries: (): Promise<BalanceSummary[]> => invoke("get_balance_summaries"),
  refreshProviders: (): Promise<any[]> => invoke("refresh_provider_balances"),
  refreshProvider: (providerId: string): Promise<any> =>
    invoke("refresh_provider_balance", { providerId }),
  history: (providerId: string, limit?: number): Promise<any[]> =>
    invoke("get_balance_history", { providerId, limit: limit ?? 30 }),
};

export const models = {
  list: (): Promise<Model[]> => invoke("list_models"),
  byPlatform: (platform: string): Promise<Model[]> =>
    invoke("get_platform_models", { platform }),
};

export const tokenCalculator = {
  fetchRemotePricing: (request: {
    base_url: string;
    api_key?: string;
  }): Promise<FetchRemoteModelPricingResponse> =>
    invoke("fetch_remote_model_pricing", { request }),
};

export const providers = {
  list: (): Promise<import("../../types").ProviderConfig[]> =>
    invoke("get_providers"),
  add: (provider: any): Promise<void> => invoke("add_provider", { provider }),
  update: (provider: any): Promise<void> =>
    invoke("update_provider", { provider }),
  switch: (id: string): Promise<void> => invoke("switch_provider", { id }),
  delete: (id: string): Promise<void> => invoke("delete_provider", { id }),
  updateOrder: (ids: string[]): Promise<void> =>
    invoke("update_providers_order", { ids }),
  streamCheck: (id: string): Promise<any> =>
    invoke("stream_check_provider", { providerId: id }),
  fetchModels: (request: {
    platform: Platform;
    base_url?: string;
    api_key_value?: string;
    api_key_id?: string;
  }): Promise<string[]> =>
    invoke("fetch_provider_models", { request }),
};
