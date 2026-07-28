import { create } from "zustand";
import { toast } from "sonner";
import {
  initRepository,
  getGitStatus,
  getGitLog,
  stageFiles,
  unstageFiles,
  commitChanges,
  rollbackCommit,
  getGitDiff,
} from "@/features/git/services";
import type { Commit, Diff, GitStatus, RollbackMode } from "@/features/git/types";

interface GitState {
  gitStatus: GitStatus | null;
  gitLog: Commit[];
  gitDiff: Diff | null;
  loading: boolean;
  error: string | null;

  init: (workspacePath: string) => Promise<boolean>;
  refresh: (workspacePath: string) => Promise<void>;
  stageFiles: (workspacePath: string, paths: string[]) => Promise<boolean>;
  unstageFiles: (workspacePath: string, paths: string[]) => Promise<boolean>;
  commit: (workspacePath: string, message: string) => Promise<string | null>;
  rollback: (workspacePath: string, hash: string, mode: RollbackMode) => Promise<boolean>;
  loadDiff: (workspacePath: string, staged: boolean, hash?: string) => Promise<void>;
  reset: () => void;
}

function getErrorText(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

export const useGit = create<GitState>((set, get) => ({
  gitStatus: null,
  gitLog: [],
  gitDiff: null,
  loading: false,
  error: null,

  init: async (workspacePath: string) => {
    set({ loading: true, error: null });
    try {
      const result = await initRepository(workspacePath);
      set({ loading: false });
      if (result.initialized) {
        toast.success("Git 仓库初始化成功");
      }
      return result.initialized;
    } catch (err) {
      const msg = getErrorText(err, "Git 仓库初始化失败");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  refresh: async (workspacePath: string) => {
    set({ loading: true, error: null });
    try {
      const [status, log] = await Promise.all([
        getGitStatus(workspacePath),
        getGitLog(workspacePath, 50),
      ]);
      set({ gitStatus: status, gitLog: log, loading: false });
    } catch (err) {
      const msg = getErrorText(err, "加载 Git 状态失败");
      set({ loading: false, error: msg });
      toast.error(msg);
    }
  },

  stageFiles: async (workspacePath: string, paths: string[]) => {
    set({ loading: true, error: null });
    try {
      await stageFiles(workspacePath, paths);
      await get().refresh(workspacePath);
      return true;
    } catch (err) {
      const msg = getErrorText(err, "暂存文件失败");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  unstageFiles: async (workspacePath: string, paths: string[]) => {
    set({ loading: true, error: null });
    try {
      await unstageFiles(workspacePath, paths);
      await get().refresh(workspacePath);
      return true;
    } catch (err) {
      const msg = getErrorText(err, "取消暂存失败");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  commit: async (workspacePath: string, message: string) => {
    if (!message.trim()) {
      toast.error("提交消息不能为空");
      return null;
    }
    set({ loading: true, error: null });
    try {
      const hash = await commitChanges(workspacePath, message);
      await get().refresh(workspacePath);
      toast.success("提交成功");
      return hash;
    } catch (err) {
      const msg = getErrorText(err, "提交失败");
      set({ loading: false, error: msg });
      toast.error(msg);
      return null;
    }
  },

  rollback: async (workspacePath: string, hash: string, mode: RollbackMode) => {
    set({ loading: true, error: null });
    try {
      await rollbackCommit(workspacePath, hash, mode);
      await get().refresh(workspacePath);
      toast.success("回滚成功");
      return true;
    } catch (err) {
      const msg = getErrorText(err, "回滚失败");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  loadDiff: async (workspacePath: string, staged: boolean, hash?: string) => {
    set({ loading: true, error: null });
    try {
      const diff = await getGitDiff(workspacePath, staged, hash);
      set({ gitDiff: diff, loading: false });
    } catch (err) {
      const msg = getErrorText(err, "加载差异失败");
      set({ loading: false, error: msg });
      toast.error(msg);
    }
  },

  reset: () => {
    set({
      gitStatus: null,
      gitLog: [],
      gitDiff: null,
      loading: false,
      error: null,
    });
  },
}));