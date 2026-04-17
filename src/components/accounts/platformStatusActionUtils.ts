import type { IdeStatusActionResult } from "../../lib/api";
import type { IdeAccount } from "../../types";

type IdePlatformAccount = Pick<IdeAccount, "origin_platform">;
type StatusActionResultLike = Pick<IdeStatusActionResult, "success" | "message" | "retried" | "attempts">;

export function supportsDailyCheckin(account: IdePlatformAccount) {
  const platform = account.origin_platform.toLowerCase();
  return platform === "codebuddy_cn" || platform === "workbuddy";
}

export function getDailyCheckinPlatformLabel(account: IdePlatformAccount) {
  return account.origin_platform === "codebuddy_cn" ? "CodeBuddy CN" : "WorkBuddy";
}

export function buildDailyCheckinFeedback(
  account: IdePlatformAccount,
  result: StatusActionResultLike,
) {
  const platformLabel = getDailyCheckinPlatformLabel(account);
  const retrySuffix = result.retried ? `（已自动重试 ${Math.max(0, result.attempts - 1)} 次）` : "";
  if (result.success) {
    return `${platformLabel} 每日签到成功${retrySuffix}${result.message ? `：${result.message}` : ""}`;
  }
  return `${platformLabel} 每日签到未完成${retrySuffix}${result.message ? `：${result.message}` : ""}`;
}
