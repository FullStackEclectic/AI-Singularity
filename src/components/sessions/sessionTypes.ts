export interface ChatSession {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  messages_count: number;
  filepath: string;
  tool_type?: string;
  cwd?: string;
  instance_id?: string;
  instance_name?: string;
  source_kind?: string;
  has_tool_calls?: boolean;
  has_log_events?: boolean;
  latest_tool_name?: string | null;
  latest_tool_status?: string | null;
}

export interface ChatMessage {
  role: string;
  content: string;
  timestamp?: number;
  full_content?: string | null;
  source_path?: string | null;
}

export interface ZombieProcess {
  pid: number;
  name: string;
  command: string;
  active_time_sec: number;
  tool_type: string;
  cwd: string;
}

export interface SessionGroup {
  cwd: string;
  label: string;
  updated_at: number;
  sessions: ChatSession[];
}

export interface CodexInstanceRecord {
  id: string;
  name: string;
  user_data_dir: string;
  extra_args?: string;
  bind_account_id?: string | null;
  bind_provider_id?: string | null;
  last_pid?: number | null;
  last_launched_at?: string | null;
  has_state_db: boolean;
  has_session_index: boolean;
  running?: boolean;
  is_default?: boolean;
  follow_local_account?: boolean;
  has_shared_skills?: boolean;
  has_shared_rules?: boolean;
  has_shared_vendor_imports_skills?: boolean;
  has_shared_agents_file?: boolean;
  has_shared_conflicts?: boolean;
  shared_conflict_paths?: string[];
  shared_strategy_version?: string;
}

export interface ProviderOption {
  id: string;
  name: string;
  tool_targets?: string | null;
  is_active?: boolean;
}

export type ActionMessage = { text: string; tone?: "error" | "success" | "info" };

export type ConfirmDialogState = {
  title: string;
  description: string;
  confirmLabel: string;
  tone?: "danger" | "primary";
  action: () => Promise<void> | void;
};

export type CodexSettingsDialogState = {
  instance: CodexInstanceRecord;
  extraArgs: string;
  bindAccountId: string;
  bindProviderId: string;
  followLocalAccount: boolean;
};
