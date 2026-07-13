import { ipc, ipcVoid } from "@/services/ipc";
import type { GitConfig } from "@/features/git/types";

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

/** 读取全局 Git 功能开关（不依赖工作区） */
export async function gitGetEnabled(): Promise<boolean> {
  return ipc<boolean>("get_git_enabled");
}

/** 设置全局 Git 功能开关 */
export async function gitSetEnabled(enabled: boolean): Promise<void> {
  return ipcVoid("set_git_enabled", { enabled });
}
