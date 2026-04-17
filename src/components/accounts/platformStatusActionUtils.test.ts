import { describe, expect, it } from "vitest";
import { buildDailyCheckinFeedback, getDailyCheckinPlatformLabel, supportsDailyCheckin } from "./platformStatusActionUtils";

describe("platformStatusActionUtils", () => {
  it("detects daily checkin support by platform", () => {
    expect(supportsDailyCheckin({ origin_platform: "codebuddy_cn" } as any)).toBe(true);
    expect(supportsDailyCheckin({ origin_platform: "workbuddy" } as any)).toBe(true);
    expect(supportsDailyCheckin({ origin_platform: "cursor" } as any)).toBe(false);
  });

  it("returns readable platform label", () => {
    expect(getDailyCheckinPlatformLabel({ origin_platform: "codebuddy_cn" } as any)).toBe("CodeBuddy CN");
    expect(getDailyCheckinPlatformLabel({ origin_platform: "workbuddy" } as any)).toBe("WorkBuddy");
  });

  it("builds success feedback with retry suffix", () => {
    const text = buildDailyCheckinFeedback(
      { origin_platform: "codebuddy_cn" } as any,
      {
        success: true,
        message: "签到成功",
        retried: true,
        attempts: 2,
      } as any,
    );

    expect(text).toContain("CodeBuddy CN 每日签到成功");
    expect(text).toContain("已自动重试 1 次");
    expect(text).toContain("签到成功");
  });

  it("builds non-success feedback without retry suffix", () => {
    const text = buildDailyCheckinFeedback(
      { origin_platform: "workbuddy" } as any,
      {
        success: false,
        message: "今日已签到",
        retried: false,
        attempts: 1,
      } as any,
    );

    expect(text).toContain("WorkBuddy 每日签到未完成");
    expect(text).not.toContain("已自动重试");
    expect(text).toContain("今日已签到");
  });
});
