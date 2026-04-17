export interface SidebarLayout {
  id: string;
  name: string;
  group_keys: string[];
  tray_platforms?: string[];
}

export const SIDEBAR_LAYOUTS_STORAGE_KEY = "ais_sidebar_layouts_v1";
export const SIDEBAR_ACTIVE_LAYOUT_STORAGE_KEY = "ais_sidebar_active_layout_id_v1";

export function sanitizeLayouts(
  raw: SidebarLayout[] | null | undefined,
  allGroupKeys: string[],
  allTrayPlatforms: string[],
  defaultLayouts: SidebarLayout[]
): SidebarLayout[] {
  if (!raw || !Array.isArray(raw) || raw.length === 0) return defaultLayouts;
  const cleaned = raw
    .map((layout) => ({
      id: String(layout.id || "").trim(),
      name: String(layout.name || "").trim() || "未命名布局",
      group_keys: Array.from(
        new Set((layout.group_keys || []).filter((key) => allGroupKeys.includes(key)))
      ),
      tray_platforms: Array.from(
        new Set((layout.tray_platforms || []).filter((key) => allTrayPlatforms.includes(key)))
      ),
    }))
    .filter((layout) => layout.id);
  if (cleaned.length === 0) return defaultLayouts;
  return cleaned.map((layout) =>
    layout.group_keys.length > 0 ? layout : { ...layout, group_keys: [...allGroupKeys] }
  );
}

export function loadSidebarLayouts(
  allGroupKeys: string[],
  allTrayPlatforms: string[],
  defaultLayouts: SidebarLayout[],
  storage?: Pick<Storage, "getItem">
): SidebarLayout[] {
  try {
    const adapter = storage ?? (typeof localStorage !== "undefined" ? localStorage : undefined);
    const raw = adapter?.getItem(SIDEBAR_LAYOUTS_STORAGE_KEY);
    if (!raw) return defaultLayouts;
    const parsed = JSON.parse(raw) as SidebarLayout[];
    return sanitizeLayouts(parsed, allGroupKeys, allTrayPlatforms, defaultLayouts);
  } catch {
    return defaultLayouts;
  }
}

export function pickActiveLayoutId(
  layouts: SidebarLayout[],
  activeLayoutId: string | null | undefined
): string {
  if (activeLayoutId && layouts.some((layout) => layout.id === activeLayoutId)) {
    return activeLayoutId;
  }
  return layouts[0]?.id || "";
}
