import { describe, expect, it } from "vitest";
import {
  loadSidebarLayouts,
  pickActiveLayoutId,
  sanitizeLayouts,
  type SidebarLayout,
} from "./sidebarLayoutStore";

const ALL_GROUP_KEYS = ["overview", "gateway", "analytics"];
const ALL_TRAY_PLATFORMS = ["open_ai", "anthropic", "gemini", "codex"];
const DEFAULT_LAYOUTS: SidebarLayout[] = [
  {
    id: "layout-default",
    name: "默认布局",
    group_keys: [...ALL_GROUP_KEYS],
    tray_platforms: [],
  },
];

describe("sidebarLayoutStore", () => {
  it("falls back to defaults when layouts are empty", () => {
    expect(sanitizeLayouts([], ALL_GROUP_KEYS, ALL_TRAY_PLATFORMS, DEFAULT_LAYOUTS)).toEqual(
      DEFAULT_LAYOUTS
    );
  });

  it("removes unknown group keys and deduplicates", () => {
    const sanitized = sanitizeLayouts(
      [
        {
          id: "layout-a",
          name: "A",
          group_keys: ["overview", "overview", "unknown", "gateway"],
          tray_platforms: ["open_ai", "open_ai", "invalid"],
        },
      ],
      ALL_GROUP_KEYS,
      ALL_TRAY_PLATFORMS,
      DEFAULT_LAYOUTS
    );
    expect(sanitized[0].group_keys).toEqual(["overview", "gateway"]);
    expect(sanitized[0].tray_platforms).toEqual(["open_ai"]);
  });

  it("loads from storage and sanitizes payload", () => {
    const fakeStorage = {
      getItem: () =>
        JSON.stringify([
          {
            id: "layout-b",
            name: "B",
            group_keys: ["analytics", "unknown"],
            tray_platforms: ["codex", "unknown"],
          },
        ]),
    };
    const loaded = loadSidebarLayouts(
      ALL_GROUP_KEYS,
      ALL_TRAY_PLATFORMS,
      DEFAULT_LAYOUTS,
      fakeStorage
    );
    expect(loaded).toEqual([
      { id: "layout-b", name: "B", group_keys: ["analytics"], tray_platforms: ["codex"] },
    ]);
  });

  it("picks first layout id when active id is invalid", () => {
    const picked = pickActiveLayoutId(DEFAULT_LAYOUTS, "missing");
    expect(picked).toBe("layout-default");
  });
});
