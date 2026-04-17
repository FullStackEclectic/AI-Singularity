import { beforeEach, describe, expect, it } from "vitest";
import {
  __accountViewTestables,
  groupAccountsByTag,
  persistAccountViewMode,
  persistTagGrouping,
  readPersistedAccountViewMode,
  readPersistedTagGrouping,
} from "./accountViewUtils";

function createMemoryStorage() {
  const store = new Map<string, string>();
  return {
    getItem: (key: string) => (store.has(key) ? store.get(key)! : null),
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
}

describe("accountViewUtils", () => {
  let storage: ReturnType<typeof createMemoryStorage>;

  beforeEach(() => {
    storage = createMemoryStorage();
  });

  it("returns list when persisted view mode is missing or invalid", () => {
    expect(readPersistedAccountViewMode(storage)).toBe("list");
    storage.setItem("ais.accounts.view_mode", "invalid");
    expect(readPersistedAccountViewMode(storage)).toBe("list");
  });

  it("persists and restores account view mode", () => {
    persistAccountViewMode("grid", storage);
    expect(readPersistedAccountViewMode(storage)).toBe("grid");
    persistAccountViewMode("compact", storage);
    expect(readPersistedAccountViewMode(storage)).toBe("compact");
  });

  it("persists and restores tag grouping toggle", () => {
    expect(readPersistedTagGrouping(storage)).toBe(false);
    persistTagGrouping(true, storage);
    expect(readPersistedTagGrouping(storage)).toBe(true);
    persistTagGrouping(false, storage);
    expect(readPersistedTagGrouping(storage)).toBe(false);
  });

  it("normalizes tags and groups accounts by primary tag", () => {
    const items = [
      { type: "ide", data: { id: "1", tags: [" Beta ", "alpha"] } },
      { type: "ide", data: { id: "2", tags: [] } },
      { type: "api", data: { id: "3" } },
      { type: "ide", data: { id: "4", tags: ["alpha", "Alpha"] } },
    ] as any[];

    const groups = groupAccountsByTag(items as any);
    expect(groups.map((group) => group.label)).toEqual(["#alpha", "未打标签", "API 资产"]);
    expect(groups[0].items.map((item: any) => item.data.id)).toEqual(["1", "4"]);
    expect(__accountViewTestables.normalizeTags([" A ", "a", "B "])).toEqual(["a", "b"]);
  });
});
