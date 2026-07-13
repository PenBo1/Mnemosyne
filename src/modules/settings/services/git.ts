import { ipc, ipcVoid } from "@/infrastructure/api";
import type { GitConfig } from "@/shared/types";

export async function gitGetConfig(workspacePath: string): Promise<GitConfig> {
  return ipc<GitConfig>("git_get_config", { workspacePath });
}

export async function gitSetConfig(
  workspacePath: string,
  config: GitConfig
): Promise<void> {
  return ipcVoid("git_set_config", { workspacePath, config });
}

export async function gitCheckInstalled(): Promise<boolean> {
  return ipc<boolean>("git_check_installed");
}
