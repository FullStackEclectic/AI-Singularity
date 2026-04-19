import type { GeminiInstanceRecord } from "../../lib/api";

export type GeminiInstanceWarning = {
  tone: "warning" | "info" | "success";
  text: string;
};

export type GeminiQuickUpdatePatch = {
  extraArgs?: string | null;
  bindAccountId?: string | null;
  projectId?: string | null;
  followLocalAccount?: boolean | null;
};

export type GeminiEditDialogState = {
  instance: GeminiInstanceRecord;
  extraArgs: string;
  bindAccountId: string;
  projectId: string;
  followLocalAccount: boolean;
};

export type UpdateProgressState = {
  phase: "idle" | "checking" | "downloading" | "installing" | "finished";
  downloaded: number;
  total: number;
};
