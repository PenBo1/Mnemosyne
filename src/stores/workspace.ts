import { create } from "zustand";
import { toast } from "sonner";
import type { WorkspaceState, Workspace } from "@/types";
import * as workspaceService from "@/services/workspaces";

// Optimistic update helper
function generateTempId(): string {
  return `temp_ws_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

export const useWorkspaceStore = create<WorkspaceState>((set, _get) => ({
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

  // Optimistic add workspace (P2 - immediate UI update)
  addWorkspace: async (name: string, path?: string) => {
    const tempId = generateTempId();
    const optimisticWorkspace: Workspace = {
      id: tempId,
      name,
      path: path || "",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    // Optimistic update: immediately show in UI
    set((state) => ({
      workspaces: [optimisticWorkspace, ...state.workspaces],
      error: null,
    }));

    try {
      const workspace = await workspaceService.createWorkspace({ name, path });
      // Replace optimistic data with real data
      set((state) => ({
        workspaces: state.workspaces.map((ws) =>
          ws.id === tempId ? workspace : ws
        ),
      }));
    } catch (err) {
      // Rollback optimistic update
      set((state) => ({
        workspaces: state.workspaces.filter((ws) => ws.id !== tempId),
        error: err instanceof Error ? err.message : "Failed to create workspace",
      }));
      toast.error("Failed to create workspace");
      throw err;
    }
  },

  // Optimistic remove workspace (P2 - immediate UI update)
  removeWorkspace: async (id: string) => {
    // Optimistic update: immediately remove from UI
    const previousWorkspaces = _get().workspaces;
    const previousActiveId = _get().activeWorkspaceId;

    set((state) => ({
      workspaces: state.workspaces.filter((ws) => ws.id !== id),
      activeWorkspaceId:
        state.activeWorkspaceId === id ? null : state.activeWorkspaceId,
      error: null,
    }));

    try {
      await workspaceService.deleteWorkspace(id);
    } catch (err) {
      // Rollback on failure
      set({
        workspaces: previousWorkspaces,
        activeWorkspaceId: previousActiveId,
        error: err instanceof Error ? err.message : "Failed to delete workspace",
      });
      toast.error("Failed to delete workspace");
    }
  },

  setActiveWorkspace: (id: string) => set({ activeWorkspaceId: id }),
}));
