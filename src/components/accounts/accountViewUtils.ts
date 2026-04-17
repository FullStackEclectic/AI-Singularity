import type { ApiKey, IdeAccount } from "../../types";

export type AccountViewMode = "list" | "grid" | "compact";

export type UnifiedAccountItemForView =
  | { type: "api"; data: ApiKey }
  | { type: "ide"; data: IdeAccount };

export type AccountTagGroup<T> = {
  key: string;
  label: string;
  items: T[];
};

type StorageLike = Pick<Storage, "getItem" | "setItem">;

const VIEW_MODE_STORAGE_KEY = "ais.accounts.view_mode";
const TAG_GROUPING_STORAGE_KEY = "ais.accounts.group_by_tag";

const VIEW_MODES: AccountViewMode[] = ["list", "grid", "compact"];

function resolveStorage(storage?: StorageLike | null): StorageLike | null {
  if (storage) return storage;
  if (typeof globalThis === "undefined") return null;
  const maybeStorage = (globalThis as Record<string, unknown>).localStorage as StorageLike | undefined;
  return maybeStorage ?? null;
}

export function readPersistedAccountViewMode(storage?: StorageLike | null): AccountViewMode {
  const resolvedStorage = resolveStorage(storage);
  if (!resolvedStorage) return "list";
  try {
    const raw = resolvedStorage.getItem(VIEW_MODE_STORAGE_KEY);
    if (!raw) return "list";
    return VIEW_MODES.includes(raw as AccountViewMode) ? (raw as AccountViewMode) : "list";
  } catch {
    return "list";
  }
}

export function persistAccountViewMode(mode: AccountViewMode, storage?: StorageLike | null) {
  const resolvedStorage = resolveStorage(storage);
  if (!resolvedStorage) return;
  try {
    resolvedStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  } catch {
    // Ignore quota/security errors; view mode is non-critical.
  }
}

export function readPersistedTagGrouping(storage?: StorageLike | null): boolean {
  const resolvedStorage = resolveStorage(storage);
  if (!resolvedStorage) return false;
  try {
    return resolvedStorage.getItem(TAG_GROUPING_STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

export function persistTagGrouping(enabled: boolean, storage?: StorageLike | null) {
  const resolvedStorage = resolveStorage(storage);
  if (!resolvedStorage) return;
  try {
    resolvedStorage.setItem(TAG_GROUPING_STORAGE_KEY, enabled ? "1" : "0");
  } catch {
    // Ignore quota/security errors; grouping preference is non-critical.
  }
}

function normalizeTags(tags?: string[]) {
  if (!Array.isArray(tags)) return [];
  return [...new Set(tags.map((tag) => tag.trim().toLowerCase()).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b)
  );
}

function resolveGroupMeta(item: UnifiedAccountItemForView) {
  if (item.type === "api") {
    return { key: "__api__", label: "API 资产" };
  }
  const tags = normalizeTags(item.data.tags);
  if (tags.length === 0) {
    return { key: "__untagged__", label: "未打标签" };
  }
  const tag = tags[0];
  return { key: `tag:${tag}`, label: `#${tag}` };
}

export function groupAccountsByTag<T extends UnifiedAccountItemForView>(items: T[]): AccountTagGroup<T>[] {
  const byKey = new Map<string, AccountTagGroup<T>>();
  const orderedKeys: string[] = [];

  for (const item of items) {
    const group = resolveGroupMeta(item);
    if (!byKey.has(group.key)) {
      byKey.set(group.key, { key: group.key, label: group.label, items: [] });
      orderedKeys.push(group.key);
    }
    byKey.get(group.key)!.items.push(item);
  }

  return orderedKeys.map((key) => byKey.get(key)!);
}

export const __accountViewTestables = {
  normalizeTags,
  resolveGroupMeta,
};
