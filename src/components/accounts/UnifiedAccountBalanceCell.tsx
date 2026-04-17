import type { ApiKey, Balance, IdeAccount } from "../../types";
import {
  formatCodebuddySummary,
  formatCodebuddyTooltip,
  formatCodexQuotaSummary,
  formatCodexQuotaTooltip,
  formatCursorSummary,
  formatCursorTooltip,
  formatGeminiQuotaSummary,
  formatGeminiQuotaTooltip,
  formatKiroSummary,
  formatKiroTooltip,
  formatQoderSummary,
  formatQoderTooltip,
  formatTraeSummary,
  formatTraeTooltip,
  formatWindsurfSummary,
  formatWindsurfTooltip,
} from "./unifiedAccountsUtils";

type UnifiedAccountItemForBalance =
  | { type: "api"; data: ApiKey; balance?: Balance }
  | { type: "ide"; data: IdeAccount; balance?: Balance };

export default function UnifiedAccountBalanceCell({
  item,
  privacy,
}: {
  item: UnifiedAccountItemForBalance;
  privacy: boolean;
}) {
  if (item.type === "api" && item.balance) {
    return (
      <span className="text-success" style={{ fontWeight: 600 }}>
        {privacy
          ? "***"
          : item.balance.balance_usd != null
            ? `$${item.balance.balance_usd.toFixed(2)}`
            : `¥${item.balance.balance_cny?.toFixed(2) || "0.00"}`}
      </span>
    );
  }

  if (item.type !== "ide") {
    return <span className="text-muted">—</span>;
  }

  if (item.data.origin_platform === "gemini" && item.data.quota_json) {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatGeminiQuotaTooltip(item.data.quota_json)}
      >
        {privacy ? "***" : formatGeminiQuotaSummary(item.data.quota_json)}
      </span>
    );
  }

  if (item.data.origin_platform === "codex" && item.data.quota_json) {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatCodexQuotaTooltip(item.data.quota_json)}
      >
        {privacy ? "***" : formatCodexQuotaSummary(item.data.quota_json)}
      </span>
    );
  }

  if (item.data.origin_platform === "cursor") {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatCursorTooltip(item.data)}
      >
        {privacy ? "***" : formatCursorSummary(item.data)}
      </span>
    );
  }

  if (item.data.origin_platform === "windsurf") {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatWindsurfTooltip(item.data)}
      >
        {privacy ? "***" : formatWindsurfSummary(item.data)}
      </span>
    );
  }

  if (item.data.origin_platform === "kiro") {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatKiroTooltip(item.data)}
      >
        {privacy ? "***" : formatKiroSummary(item.data)}
      </span>
    );
  }

  if (item.data.origin_platform === "qoder") {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatQoderTooltip(item.data)}
      >
        {privacy ? "***" : formatQoderSummary(item.data)}
      </span>
    );
  }

  if (item.data.origin_platform === "trae") {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatTraeTooltip(item.data)}
      >
        {privacy ? "***" : formatTraeSummary(item.data)}
      </span>
    );
  }

  if (
    item.data.origin_platform === "codebuddy" ||
    item.data.origin_platform === "codebuddy_cn" ||
    item.data.origin_platform === "workbuddy"
  ) {
    return (
      <span
        className="text-success"
        style={{ fontWeight: 600 }}
        title={privacy ? undefined : formatCodebuddyTooltip(item.data)}
      >
        {privacy ? "***" : formatCodebuddySummary(item.data)}
      </span>
    );
  }

  return <span className="text-muted">—</span>;
}
