import { invoke } from "@tauri-apps/api/core";
import type {
  DashboardStats,
  EnvConflict,
  TokenUsageStat,
} from "../../types";
import type {
  SkillStorageInfo,
} from "./types";

export const stats = {
  getDashboard: (): Promise<DashboardStats> => invoke("get_dashboard_stats"),
  getTokenUsage: (): Promise<{ by_app: TokenUsageStat[]; by_model: TokenUsageStat[] }> =>
    invoke("get_token_usage_stats"),
};

export const env = {
  checkConflicts: (appName: string): Promise<EnvConflict[]> =>
    invoke("check_system_env_conflicts", { appName }),
};

export const proxy = {
  syncEngineConfig: (config: any): Promise<void> =>
    invoke("sync_proxy_engine_config", { config }),
};

export const security = {
  getAccessLogs: (limit?: number): Promise<any[]> =>
    invoke("get_ip_access_logs", { limit }),
  clearAccessLogs: (): Promise<void> => invoke("clear_ip_access_logs"),
  getRules: (): Promise<any[]> => invoke("get_ip_rules"),
  addRule: (ipCidr: string, ruleType: string, notes?: string): Promise<void> =>
    invoke("add_ip_rule", { ipCidr, ruleType, notes }),
  deleteRule: (id: string): Promise<void> => invoke("delete_ip_rule", { id }),
  toggleRule: (id: string, active: boolean): Promise<void> =>
    invoke("toggle_ip_rule", { id, active }),
};

export const alerts = {
  get: (): Promise<any[]> => invoke("get_alerts"),
};

export const speedtest = {
  run: (): Promise<any[]> => invoke("run_speedtest"),
};

export const tools = {
  checkStatus: (toolId: string): Promise<any> =>
    invoke("check_tool_status", { toolId }),
  deploy: (toolId: string): Promise<void> =>
    invoke("deploy_tool", { toolId }),
};

export const skills = {
  getStorageInfo: (): Promise<SkillStorageInfo> =>
    invoke("get_skill_storage_info"),
};
