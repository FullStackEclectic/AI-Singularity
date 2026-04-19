import type {
  AccountGroup,
  Platform,
} from "../../types";

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
  follow_local_account?: boolean;
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

export interface TokenCalculatorRemoteModelPricing {
  id: string;
  name?: string | null;
  description?: string | null;
  input_price_per_1m?: number | null;
  output_price_per_1m?: number | null;
  cache_read_price_per_1m?: number | null;
  fixed_price_usd?: number | null;
  quota_type?: number | null;
  model_ratio?: number | null;
  completion_ratio?: number | null;
  cache_ratio?: number | null;
  model_price?: number | null;
  enable_groups?: string[];
  vendor_id?: number | null;
  recommended_group?: string | null;
}

export interface FetchRemoteModelPricingResponse {
  models: TokenCalculatorRemoteModelPricing[];
  source_endpoint: string;
  warnings: string[];
  provider_kind?: string | null;
  quota_per_unit?: number | null;
  group_ratios: Record<string, number>;
  group_labels: Record<string, string>;
  auto_groups: string[];
}

export interface UpdateSettings {
  auto_check: boolean;
  auto_install: boolean;
  skip_version?: string | null;
  disable_reminders?: boolean;
  silent_reminder_strategy?: "immediate" | "daily" | "weekly" | string;
  last_reminded_at?: string | null;
  last_reminded_version?: string | null;
  last_check_at?: string | null;
}

export interface UpdateReminderDecision {
  should_notify: boolean;
  reason: string;
  settings: UpdateSettings;
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

export interface CurrentAccountSnapshot {
  platform: string;
  account_id?: string | null;
  label?: string | null;
  email?: string | null;
  status?: string | null;
}

export interface IdeStatusActionResult {
  account_id: string;
  platform: string;
  action: string;
  success: boolean;
  message: string;
  reward?: Record<string, unknown> | null;
  next_checkin_in?: number | null;
  attempts: number;
  retried: boolean;
  retryable: boolean;
  executed_at: string;
}

export type FloatingAccountCardScope = "global" | "instance";

export interface FloatingAccountCard {
  id: string;
  scope: FloatingAccountCardScope;
  instance_id?: string | null;
  title: string;
  bound_platforms: string[];
  window_label?: string | null;
  always_on_top: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
  collapsed: boolean;
  visible: boolean;
  updated_at: string;
}

export interface CreateFloatingAccountCardRequest {
  scope?: FloatingAccountCardScope;
  instance_id?: string | null;
  title?: string | null;
  bound_platforms?: string[];
  window_label?: string | null;
  always_on_top?: boolean;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  collapsed?: boolean;
  visible?: boolean;
}

export interface UpdateFloatingAccountCardPatch {
  scope?: FloatingAccountCardScope;
  instance_id?: string | null;
  title?: string;
  bound_platforms?: string[];
  window_label?: string | null;
  always_on_top?: boolean;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  collapsed?: boolean;
  visible?: boolean;
}

export interface WebReportStatus {
  running: boolean;
  port?: number | null;
  local_url?: string | null;
  health_url?: string | null;
  status_api_url?: string | null;
  snapshot_api_url?: string | null;
  auth_enabled: boolean;
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
  trigger_mode?: string;
  reset_window?: string;
  window_day_policy?: string;
  window_fallback_policy?: string;
  client_version_mode?: string;
  client_version_fallback_mode?: string;
  command_template: string;
  model: string;
  prompt: string;
  cron: string;
  notes?: string | null;
  timeout_seconds: number;
  retry_failed_times?: number;
  pause_after_failures?: number;
  created_at: string;
  updated_at: string;
  last_run_at?: string | null;
  last_status?: string | null;
  last_category?: string | null;
  last_message?: string | null;
  consecutive_failures?: number;
}

export interface WakeupState {
  enabled: boolean;
  tasks: WakeupTask[];
}

export interface WakeupHistoryItem {
  id: string;
  run_id?: string | null;
  task_id?: string | null;
  task_name: string;
  account_id: string;
  model: string;
  status: string;
  category?: string;
  message?: string | null;
  created_at: string;
}

export interface WakeupVerificationBatchItem {
  account_id: string;
  email: string;
  status: string;
  category: string;
  attempts: number;
  message: string;
}

export interface WakeupCategoryCount {
  category: string;
  count: number;
}

export interface WakeupVerificationBatchResult {
  executed_count: number;
  success_count: number;
  failed_count: number;
  retried_count: number;
  canceled: boolean;
  category_counts: WakeupCategoryCount[];
  items: WakeupVerificationBatchItem[];
}

export type { AccountGroup };
