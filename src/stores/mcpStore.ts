import { create } from "zustand";
import type { McpServer } from "../types";
import { api } from "../lib/api";

interface McpState {
  servers: McpServer[];
  isLoading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  add: (server: McpServer) => Promise<void>;
  toggle: (id: string, isActive: boolean) => Promise<void>;
  deleteMcp: (id: string) => Promise<void>;
}

export const useMcpStore = create<McpState>((set, get) => ({
  servers: [],
  isLoading: false,
  error: null,

  fetch: async () => {
    if (get().isLoading) return;
    set({ isLoading: true, error: null });
    try {
      const servers = await api.mcp.list();
      set({ servers, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  add: async (server: McpServer) => {
    try {
      await api.mcp.add(server);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  toggle: async (id: string, isActive: boolean) => {
    try {
      await api.mcp.toggle(id, isActive);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteMcp: async (id: string) => {
    try {
      await api.mcp.delete(id);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
}));
