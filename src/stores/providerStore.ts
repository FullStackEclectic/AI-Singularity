import { create } from "zustand";
import type { ProviderConfig } from "../types";
import { api } from "../lib/api";

interface ProviderState {
  providers: ProviderConfig[];
  isLoading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  add: (provider: Omit<ProviderConfig, "id" | "created_at" | "updated_at"> & { id?: string }) => Promise<void>;
  update: (provider: ProviderConfig) => Promise<void>;
  switchProvider: (id: string) => Promise<void>;
  deleteProvider: (id: string) => Promise<void>;
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  providers: [],
  isLoading: false,
  error: null,

  fetch: async () => {
    if (get().isLoading) return;
    set({ isLoading: true, error: null });
    try {
      const providers = await api.providers.list();
      set({ providers, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  add: async (provider) => {
    try {
      await api.providers.add(provider);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  update: async (provider: ProviderConfig) => {
    try {
      await api.providers.update(provider);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  switchProvider: async (id: string) => {
    try {
      await api.providers.switch(id);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteProvider: async (id: string) => {
    try {
      await api.providers.delete(id);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
}));
