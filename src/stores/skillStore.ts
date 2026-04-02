import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface SkillInfo {
  id: string;
  name: string;
  source_url: string | null;
  local_path: string;
  status: string;
}

interface SkillState {
  skills: SkillInfo[];
  isLoading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  install: (url: string) => Promise<void>;
  update: (id: string) => Promise<void>;
  uninstall: (id: string) => Promise<void>;
}

export const useSkillStore = create<SkillState>((set, get) => ({
  skills: [],
  isLoading: false,
  error: null,

  fetch: async () => {
    set({ isLoading: true, error: null });
    try {
      const data = await invoke<SkillInfo[]>("list_skills");
      set({ skills: data, isLoading: false });
    } catch (err: any) {
      set({ error: String(err), isLoading: false });
    }
  },

  install: async (url: string) => {
    try {
      await invoke("install_skill", { url });
      await get().fetch();
    } catch (err: any) {
      throw err;
    }
  },

  update: async (id: string) => {
    try {
      await invoke("update_skill", { id });
      await get().fetch();
    } catch (err: any) {
      throw err;
    }
  },

  uninstall: async (id: string) => {
    try {
      await invoke("uninstall_skill", { id });
      await get().fetch();
    } catch (err: any) {
      throw err;
    }
  }
}));
