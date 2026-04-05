/**
 * 账号分组服务 — 基于 localStorage 的轻量分组管理
 * 分组数据仅用于 UI 展示层，不影响后端轮询逻辑
 */

export interface AccountGroup {
  id: string;
  name: string;
  /** account id 列表（ide_accounts 和 api_keys 混用） */
  accountIds: string[];
  order: number;
  createdAt: number;
}

const STORAGE_KEY = 'ai_singularity.account_groups';

function loadGroups(): AccountGroup[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as AccountGroup[];
  } catch {
    return [];
  }
}

function saveGroups(groups: AccountGroup[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(groups));
  } catch (e) {
    console.error('[GroupService] 保存分组失败', e);
  }
}

export function getGroups(): AccountGroup[] {
  return loadGroups().sort((a, b) => a.order - b.order);
}

export function createGroup(name: string): AccountGroup {
  const groups = loadGroups();
  const newGroup: AccountGroup = {
    id: `group_${Date.now()}`,
    name: name.trim(),
    accountIds: [],
    order: groups.length,
    createdAt: Date.now(),
  };
  saveGroups([...groups, newGroup]);
  return newGroup;
}

export function renameGroup(id: string, name: string): void {
  const groups = loadGroups().map(g =>
    g.id === id ? { ...g, name: name.trim() } : g
  );
  saveGroups(groups);
}

export function deleteGroup(id: string): void {
  const groups = loadGroups().filter(g => g.id !== id);
  saveGroups(groups);
}

export function assignAccountsToGroup(groupId: string, accountIds: string[]): void {
  const groups = loadGroups().map(g => {
    if (g.id !== groupId) return g;
    const existing = new Set(g.accountIds);
    accountIds.forEach(id => existing.add(id));
    return { ...g, accountIds: Array.from(existing) };
  });
  saveGroups(groups);
}

export function removeAccountsFromGroup(groupId: string, accountIds: string[]): void {
  const toRemove = new Set(accountIds);
  const groups = loadGroups().map(g => {
    if (g.id !== groupId) return g;
    return { ...g, accountIds: g.accountIds.filter(id => !toRemove.has(id)) };
  });
  saveGroups(groups);
}

export function updateGroupOrder(orderedIds: string[]): void {
  const groups = loadGroups();
  const orderMap = new Map(orderedIds.map((id, i) => [id, i]));
  const reordered = groups.map(g => ({
    ...g,
    order: orderMap.has(g.id) ? orderMap.get(g.id)! : g.order,
  }));
  saveGroups(reordered);
}

/** 获取某个账号所在的分组 */
export function getGroupsForAccount(accountId: string): AccountGroup[] {
  return loadGroups().filter(g => g.accountIds.includes(accountId));
}

/** 获取未分配到任何分组的账号 IDs */
export function getUngroupedIds(allIds: string[]): string[] {
  const assignedIds = new Set(loadGroups().flatMap(g => g.accountIds));
  return allIds.filter(id => !assignedIds.has(id));
}
