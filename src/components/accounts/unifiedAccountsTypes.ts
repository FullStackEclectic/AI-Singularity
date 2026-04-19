import type { ApiKey, Balance, IdeAccount } from "../../types";

export type ChannelType = "api" | "ide" | "all";

export interface ChannelMeta {
  id: string;
  type: ChannelType;
  label: string;
  count: number;
}

export type ActionMessage = {
  text: string;
  tone?: "success" | "error" | "info";
};

export type AccountGroupDialogState = {
  mode: "manage" | "assign";
  ids: string[];
  count: number;
  channelLabel: string;
};

export type UnifiedAccountItem =
  | { type: "api"; data: ApiKey; balance?: Balance }
  | { type: "ide"; data: IdeAccount };

export type AccountRenderRow =
  | { type: "group"; key: string; label: string; count: number }
  | { type: "item"; key: string; item: UnifiedAccountItem };

export interface IdeOverviewSummary {
  total: number;
  platforms: string[];
  currentCount: number;
  activeCount: number;
  attentionCount: number;
  expiredCount: number;
  forbiddenCount: number;
  rateLimitedCount: number;
  proxyDisabledCount: number;
  manuallyDisabledCount: number;
  taggedCount: number;
  recentUsedCount: number;
  latestUsedAt: string;
  latestSyncAt: string;
}
