import { ipc, ipcVoid } from "@/services/ipc";
import type { Commit, Diff, GitConfig, GitInitResult, GitStatus, RollbackMode } from "@/features/git/types";

// ── 初始化 ─────────────────────────────────────────

export async function initRepository(workspacePath: string): Promise<GitInitResult> {
  return ipc<GitInitResult>("git_init", { workspacePath });
}

// ── 状态与历史 ───────────────────────────────────────

export async function getGitStatus(workspacePath: string): Promise<GitStatus> {
  return ipc<GitStatus>("git_status", { workspacePath });
}

export async function getGitLog(
  workspacePath: string,
  limit?: number,
  skip?: number
): Promise<Commit[]> {
  return ipc<Commit[]>("git_log", { workspacePath, limit, skip });
}

export async function getGitDiff(
  workspacePath: string,
  staged?: boolean,
  commitHash?: string
): Promise<Diff> {
  return ipc<Diff>("git_diff", { workspacePath, staged, commitHash });
}

// ── 变更操作 ──────────────────────────────────────────────

export async function stageFiles(workspacePath: string, paths: string[]): Promise<void> {
  await ipcVoid("git_stage", { workspacePath, paths });
}

export async function unstageFiles(workspacePath: string, paths: string[]): Promise<void> {
  await ipcVoid("git_unstage", { workspacePath, paths });
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

// ── 配置 ─────────────────────────────────────────────────

export async function getGitConfig(
  workspacePath: string,
  key?: string,
  global?: boolean
): Promise<GitConfig> {
  return ipc<GitConfig>("git_get_config", { workspacePath, key, global });
}

export async function setGitConfig(
  workspacePath: string,
  key: string,
  value: string,
  global?: boolean
): Promise<void> {
  await ipcVoid("git_set_config", { workspacePath, key, value, global });
}

// ── 分支 ─────────────────────────────────────────────────

export async function getBranches(workspacePath: string): Promise<string[]> {
  return ipc<string[]>("git_branches", { workspacePath });
}