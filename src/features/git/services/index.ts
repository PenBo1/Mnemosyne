import { ipc, ipcVoid } from "@/infrastructure/api";
import type {
  Commit,
  Diff,
  GitConfig,
  GitInitResult,
  GitStatus,
  InstallResult,
  RollbackMode,
} from "@/shared/types";

// ── Install & Init ─────────────────────────────────────────

export async function checkGitInstalled(): Promise<boolean> {
  return ipc<boolean>("git_check_installed");
}

export async function installGit(): Promise<InstallResult> {
  return ipc<InstallResult>("git_install");
}

export async function initRepository(workspacePath: string): Promise<GitInitResult> {
  return ipc<GitInitResult>("git_init", { workspacePath });
}

// ── Status & History ───────────────────────────────────────

export async function getGitStatus(workspacePath: string): Promise<GitStatus> {
  return ipc<GitStatus>("git_status", { workspacePath });
}

export async function getGitLog(
  workspacePath: string,
  limit: number | null = null
): Promise<Commit[]> {
  return ipc<Commit[]>("git_log", { workspacePath, limit });
}

export async function getGitDiff(
  workspacePath: string,
  commitHash: string | null = null
): Promise<Diff> {
  return ipc<Diff>("git_diff", { workspacePath, commitHash });
}

// ── Mutations ──────────────────────────────────────────────

export async function stageFiles(workspacePath: string, paths: string[]): Promise<void> {
  await ipcVoid("git_stage", { workspacePath, paths });
}

export async function commitChanges(workspacePath: string, message: string): Promise<string> {
  return ipc<string>("git_commit", { workspacePath, message });
}

export async function rollbackCommit(
  workspacePath: string,
  commitHash: string,
  mode: RollbackMode
): Promise<void> {
  await ipcVoid("git_rollback", { workspacePath, commitHash, mode });
}

// ── Config ─────────────────────────────────────────────────

export async function getGitConfig(workspacePath: string): Promise<GitConfig> {
  return ipc<GitConfig>("git_get_config", { workspacePath });
}

export async function setGitConfig(workspacePath: string, config: GitConfig): Promise<void> {
  await ipcVoid("git_set_config", { workspacePath, config });
}
