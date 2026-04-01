import { create } from "zustand";
import { api } from "../lib/api";

export interface PromptConfig {
  id: string;
  name: string;
  target_file: string;
  content: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

interface PromptState {
  prompts: PromptConfig[];
  isLoading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  save: (prompt: PromptConfig) => Promise<void>;
  deletePrompt: (id: string) => Promise<void>;
  syncPrompt: (id: string, workspaceDir: string) => Promise<void>;
}

export const usePromptStore = create<PromptState>((set, get) => ({
  prompts: [],
  isLoading: false,
  error: null,

  fetch: async () => {
    if (get().isLoading) return;
    set({ isLoading: true, error: null });
    try {
      const prompts = await api.prompts.list();
      set({ prompts, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  save: async (prompt) => {
    try {
      if (!prompt.id) {
        prompt.id = crypto.randomUUID();
        prompt.created_at = new Date().toISOString();
      }
      prompt.updated_at = new Date().toISOString();
      await api.prompts.save(prompt);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deletePrompt: async (id) => {
    try {
      await api.prompts.delete(id);
      await get().fetch();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  syncPrompt: async (id, workspaceDir) => {
    try {
      await api.prompts.sync(id, workspaceDir);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
}));
