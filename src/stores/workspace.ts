import { create } from "zustand";
import type { WorkspaceState } from "@/types";
import * as workspaceService from "@/services/workspaces";

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspaces: [],
  activeWorkspaceId: null,
  loading: false,
  error: null,

  loadWorkspaces: async () => {
    set({ loading: true, error: null });
    try {
      const workspaces = await workspaceService.fetchWorkspaces();
      set({ workspaces, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load workspaces";
      set({ error: message, loading: false });
    }
  },

  addWorkspace: async (name: string, path?: string) => {
    set({ error: null });
    try {
      const workspace = await workspaceService.createWorkspace({ name, path });
      set((state) => ({
        workspaces: [workspace, ...state.workspaces],
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to create workspace";
      set({ error: message });
      throw err;
    }
  },

  removeWorkspace: async (id: string) => {
    set({ error: null });
    try {
      await workspaceService.deleteWorkspace(id);
      set((state) => ({
        workspaces: state.workspaces.filter((ws) => ws.id !== id),
        activeWorkspaceId:
          state.activeWorkspaceId === id ? null : state.activeWorkspaceId,
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to delete workspace";
      set({ error: message });
      throw err;
    }
  },

  setActiveWorkspace: (id: string) => set({ activeWorkspaceId: id }),
}));
