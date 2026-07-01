import { create } from "zustand";
import { toast } from "sonner";
import * as gitService from "@/features/git/services";
import type { Commit, Diff, GitStatus, RollbackMode } from "@/shared/types";

interface GitState {
  gitInstalled: boolean | null;
  gitVersion: string | null;
  gitStatus: GitStatus | null;
  gitLog: Commit[];
  gitDiff: Diff | null;
  loading: boolean;
  error: string | null;

  checkInstalled: () => Promise<boolean>;
  install: () => Promise<boolean>;
  init: (workspacePath: string) => Promise<boolean>;
  refresh: (workspacePath: string) => Promise<void>;
  stageFiles: (workspacePath: string, paths: string[]) => Promise<boolean>;
  commit: (workspacePath: string, message: string) => Promise<string | null>;
  rollback: (workspacePath: string, hash: string, mode: RollbackMode) => Promise<boolean>;
  loadDiff: (workspacePath: string, hash: string | null) => Promise<void>;
  reset: () => void;
}

function getErrorText(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

export const useGit = create<GitState>((set, get) => ({
  gitInstalled: null,
  gitVersion: null,
  gitStatus: null,
  gitLog: [],
  gitDiff: null,
  loading: false,
  error: null,

  checkInstalled: async () => {
    set({ loading: true, error: null });
    try {
      const installed = await gitService.checkGitInstalled();
      set({ gitInstalled: installed, loading: false });
      return installed;
    } catch (err) {
      set({
        gitInstalled: false,
        loading: false,
        error: getErrorText(err, "Failed to check Git installation"),
      });
      return false;
    }
  },

  install: async () => {
    set({ loading: true, error: null });
    try {
      const result = await gitService.installGit();
      if (result.success) {
        set({
          gitInstalled: true,
          gitVersion: result.version,
          loading: false,
        });
        toast.success(result.message || "Git installed successfully");
      } else {
        set({ loading: false });
        toast.error(result.message || "Failed to install Git");
      }
      return result.success;
    } catch (err) {
      const msg = getErrorText(err, "Failed to install Git");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  init: async (workspacePath: string) => {
    set({ loading: true, error: null });
    try {
      const result = await gitService.initRepository(workspacePath);
      set({ loading: false });
      if (result.initialized) {
        toast.success("Git repository initialized");
      }
      return result.initialized;
    } catch (err) {
      const msg = getErrorText(err, "Failed to initialize Git repository");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  refresh: async (workspacePath: string) => {
    set({ loading: true, error: null });
    try {
      const [status, log] = await Promise.all([
        gitService.getGitStatus(workspacePath),
        gitService.getGitLog(workspacePath, 50),
      ]);
      set({ gitStatus: status, gitLog: log, loading: false });
    } catch (err) {
      const msg = getErrorText(err, "Failed to load Git status");
      set({ loading: false, error: msg });
      toast.error(msg);
    }
  },

  stageFiles: async (workspacePath: string, paths: string[]) => {
    set({ loading: true, error: null });
    try {
      await gitService.stageFiles(workspacePath, paths);
      await get().refresh(workspacePath);
      return true;
    } catch (err) {
      const msg = getErrorText(err, "Failed to stage files");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  commit: async (workspacePath: string, message: string) => {
    if (!message.trim()) {
      toast.error("Commit message cannot be empty");
      return null;
    }
    set({ loading: true, error: null });
    try {
      const hash = await gitService.commitChanges(workspacePath, message);
      await get().refresh(workspacePath);
      toast.success("Commit successful");
      return hash;
    } catch (err) {
      const msg = getErrorText(err, "Failed to commit");
      set({ loading: false, error: msg });
      toast.error(msg);
      return null;
    }
  },

  rollback: async (workspacePath: string, hash: string, mode: RollbackMode) => {
    set({ loading: true, error: null });
    try {
      await gitService.rollbackCommit(workspacePath, hash, mode);
      await get().refresh(workspacePath);
      toast.success("Rollback successful");
      return true;
    } catch (err) {
      const msg = getErrorText(err, "Failed to rollback");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  loadDiff: async (workspacePath: string, hash: string | null) => {
    set({ loading: true, error: null });
    try {
      const diff = await gitService.getGitDiff(workspacePath, hash);
      set({ gitDiff: diff, loading: false });
    } catch (err) {
      const msg = getErrorText(err, "Failed to load diff");
      set({ loading: false, error: msg });
      toast.error(msg);
    }
  },

  reset: () => {
    set({
      gitInstalled: null,
      gitVersion: null,
      gitStatus: null,
      gitLog: [],
      gitDiff: null,
      loading: false,
      error: null,
    });
  },
}));
