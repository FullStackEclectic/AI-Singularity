import { invoke } from "@tauri-apps/api/core";
import type {
  AnnouncementState,
  DesktopLogFile,
  DesktopLogReadResult,
  LinuxInstallResult,
  LinuxReleaseInfo,
  UpdateReminderDecision,
  UpdateRuntimeInfo,
  UpdateSettings,
  WebReportStatus,
  WebSocketStatus,
} from "./types";

export const logs = {
  list: (): Promise<DesktopLogFile[]> => invoke("list_desktop_logs"),
  read: (name: string, lines?: number, query?: string): Promise<DesktopLogReadResult> =>
    invoke("read_desktop_log", { name, lines: lines ?? 500, query: query?.trim() || null }),
  export: (name: string, destination: string): Promise<void> =>
    invoke("export_desktop_log", { name, destination }),
};

export const update = {
  getSettings: (): Promise<UpdateSettings> => invoke("get_update_settings"),
  saveSettings: (settings: UpdateSettings): Promise<void> =>
    invoke("save_update_settings", { settings }),
  markCheckedNow: (): Promise<UpdateSettings> => invoke("update_last_check_time"),
  markReminded: (version: string): Promise<UpdateSettings> =>
    invoke("mark_update_reminded", { version }),
  evaluateReminderPolicy: (version: string): Promise<UpdateReminderDecision> =>
    invoke("evaluate_update_reminder_policy", { version }),
  getRuntimeInfo: (): Promise<UpdateRuntimeInfo> =>
    invoke("get_update_runtime_info"),
  getLinuxReleaseInfo: (): Promise<LinuxReleaseInfo> =>
    invoke("get_linux_update_release_info"),
  openAssetUrl: (url: string): Promise<void> =>
    invoke("open_update_asset_url", { url }),
  installLinuxAsset: (request: {
    url: string;
    kind: string;
    version?: string | null;
  }): Promise<LinuxInstallResult> =>
    invoke("install_linux_update_asset", {
      url: request.url,
      kind: request.kind,
      version: request.version ?? null,
    }),
};

export const websocket = {
  getStatus: (): Promise<WebSocketStatus> => invoke("get_websocket_status"),
};

export const webReport = {
  getPort: (): Promise<number | null> => invoke("get_web_report_port"),
  getStatus: (): Promise<WebReportStatus> => invoke("get_web_report_status"),
};

export const announcements = {
  getState: (locale?: string): Promise<AnnouncementState> =>
    invoke("announcement_get_state", { locale: locale ?? null }),
  markRead: (id: string): Promise<void> =>
    invoke("announcement_mark_as_read", { id }),
  markAllRead: (locale?: string): Promise<void> =>
    invoke("announcement_mark_all_as_read", { locale: locale ?? null }),
  refresh: (locale?: string): Promise<AnnouncementState> =>
    invoke("announcement_force_refresh", { locale: locale ?? null }),
};
