import { invoke } from "@tauri-apps/api/core";
import type { SkillStorageInfo } from "./types";

export interface SkillInfo {
  id: string;
  name: string;
  source_url: string | null;
  local_path: string;
  status: string;
}

export const skills = {
  getStorageInfo: (): Promise<SkillStorageInfo> =>
    invoke("get_skill_storage_info"),
  list: (): Promise<SkillInfo[]> =>
    invoke("list_skills"),
  install: (url: string): Promise<void> =>
    invoke("install_skill", { url }),
  update: (id: string): Promise<void> =>
    invoke("update_skill", { id }),
  uninstall: (id: string): Promise<void> =>
    invoke("uninstall_skill", { id }),
};
