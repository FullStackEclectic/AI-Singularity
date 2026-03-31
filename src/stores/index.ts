import { create } from "zustand";
import type { ApiKey } from "../types";
import { api } from "../lib/api";

interface KeysState {
  keys: ApiKey[];
  isLoading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  refresh: () => Promise<void>;
  deleteKey: (id: string) => Promise<void>;
}

export const useKeysStore = create<KeysState>((set, get) => ({
  keys: [],
  isLoading: false,
  error: null,

  fetch: async () => {
    if (get().isLoading) return;
    set({ isLoading: true, error: null });
    try {
      const keys = await api.keys.list();
      set({ keys, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  refresh: async () => {
    try {
      const keys = await api.keys.list();
      set({ keys });
    } catch {}
  },

  deleteKey: async (id: string) => {
    await api.keys.delete(id);
    set((s) => ({ keys: s.keys.filter((k) => k.id !== id) }));
  },
}));

interface UIState {
  activePage: string;
  sidebarCollapsed: boolean;
  setActivePage: (page: string) => void;
  toggleSidebar: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  activePage: "dashboard",
  sidebarCollapsed: false,
  setActivePage: (page) => set({ activePage: page }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
}));
