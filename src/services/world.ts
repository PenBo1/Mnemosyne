import { ipc } from "@/lib/ipc";
import type { WorldSetting } from "@/types";

export async function listWorldSettings(novelId: string): Promise<WorldSetting[]> {
  return ipc<WorldSetting[]>("world_setting_list", { novelId });
}

export async function createWorldSetting(params: {
  novelId: string;
  category: string;
  name: string;
  description: string;
  content: string;
  tags: string[];
}): Promise<WorldSetting> {
  return ipc<WorldSetting>("world_setting_create", params);
}

export async function updateWorldSetting(params: {
  id: string;
  name: string;
  description: string;
  content: string;
  tags: string[];
}): Promise<WorldSetting> {
  return ipc<WorldSetting>("world_setting_update", params);
}

export async function deleteWorldSetting(id: string): Promise<void> {
  await ipc<void>("world_setting_delete", { id });
}
