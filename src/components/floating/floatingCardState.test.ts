import { describe, expect, it } from "vitest";
import type { FloatingAccountCard } from "../../lib/api";
import {
  isFloatingCardConflictError,
  mergeFloatingCardByUpdatedAt,
  sortFloatingCardsForRender,
} from "./floatingCardState";

function buildCard(partial: Partial<FloatingAccountCard>): FloatingAccountCard {
  return {
    id: partial.id || "card-1",
    scope: partial.scope || "global",
    instance_id: partial.instance_id ?? null,
    title: partial.title || "Test",
    bound_platforms: partial.bound_platforms || ["codex"],
    window_label: partial.window_label ?? "main",
    always_on_top: partial.always_on_top ?? false,
    x: partial.x ?? 32,
    y: partial.y ?? 96,
    width: partial.width ?? 320,
    height: partial.height ?? 220,
    collapsed: partial.collapsed ?? false,
    visible: partial.visible ?? true,
    updated_at: partial.updated_at || "2026-04-16T00:00:00Z",
  };
}

describe("floatingCardState", () => {
  it("replaces local card when incoming update is newer", () => {
    const base = buildCard({ id: "a", title: "old", updated_at: "2026-04-16T00:00:00Z" });
    const incoming = buildCard({ id: "a", title: "new", updated_at: "2026-04-16T00:00:01Z" });
    const merged = mergeFloatingCardByUpdatedAt([base], incoming);
    expect(merged).toHaveLength(1);
    expect(merged[0].title).toBe("new");
  });

  it("keeps local card when incoming update is stale", () => {
    const base = buildCard({ id: "a", title: "local", updated_at: "2026-04-16T00:00:02Z" });
    const incoming = buildCard({ id: "a", title: "stale", updated_at: "2026-04-16T00:00:01Z" });
    const merged = mergeFloatingCardByUpdatedAt([base], incoming);
    expect(merged[0].title).toBe("local");
  });

  it("sorts always-on-top cards first", () => {
    const cards = sortFloatingCardsForRender([
      buildCard({ id: "normal", always_on_top: false, updated_at: "2026-04-16T00:00:03Z" }),
      buildCard({ id: "top", always_on_top: true, updated_at: "2026-04-16T00:00:01Z" }),
    ]);
    expect(cards[0].id).toBe("top");
  });

  it("recognizes conflict error marker", () => {
    expect(isFloatingCardConflictError("floating_card_conflict:2026-04-16T00:00:00Z")).toBe(true);
    expect(isFloatingCardConflictError(new Error("other"))).toBe(false);
  });
});
