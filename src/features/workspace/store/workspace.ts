import { create } from "zustand";
import { toast } from "sonner";
import type { WorkspaceState, Workspace } from "@/features/workspace/types";
import * as workspaceService from "@/features/workspace/services";

// 乐观更新辅助函数
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
      // 恢复上次活动工作区：取 last_opened_at 最新的（后端已按此排序）
      const activeId = workspaces.length > 0 ? workspaces[0].id : null;
      set({ workspaces, activeWorkspaceId: activeId, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load workspaces";
      set({ error: message, loading: false });
    }
  },

  // 乐观添加 workspace（P2 - 立即更新 UI）
  addWorkspace: async (name: string, path?: string) => {
    const tempId = generateTempId();
    const optimisticWorkspace: Workspace = {
      id: tempId,
      name,
      path: path || "",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      last_opened_at: null,
      is_archived: false,
      archived_at: null,
      sort_order: 0,
    };

    // 乐观更新：立即在 UI 中显示
    set((state) => ({
      workspaces: [optimisticWorkspace, ...state.workspaces],
      error: null,
    }));

    try {
      const workspace = await workspaceService.createWorkspace({ name, path });
      // 用真实数据替换乐观数据，并设为当前活动工作区
      set((state) => ({
        workspaces: state.workspaces.map((ws) =>
          ws.id === tempId ? workspace : ws
        ),
        activeWorkspaceId: workspace.id,
      }));
      // 触发 touch 持久化 last_opened_at
      void workspaceService.touchWorkspace(workspace.id);
    } catch (err) {
      // 回滚乐观更新
      set((state) => ({
        workspaces: state.workspaces.filter((ws) => ws.id !== tempId),
        error: err instanceof Error ? err.message : "Failed to create workspace",
      }));
      toast.error("Failed to create workspace");
      throw err;
    }
  },

  // 乐观移除 workspace（P2 - 立即更新 UI）
  removeWorkspace: async (id: string) => {
    // 乐观更新：立即从 UI 中移除
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
      // 失败时回滚
      set({
        workspaces: previousWorkspaces,
        activeWorkspaceId: previousActiveId,
        error: err instanceof Error ? err.message : "Failed to delete workspace",
      });
      toast.error("Failed to delete workspace");
    }
  },

  setActiveWorkspace: (id: string) => {
    set({ activeWorkspaceId: id });
    // 持久化 last_opened_at（不阻塞 UI，失败仅 console）
    void workspaceService.touchWorkspace(id).catch((e) => {
      console.error("[workspace] touchWorkspace failed", e);
    });
  },

  // 更新工作区排序顺序（乐观更新）
  updateWorkspaceSortOrder: async (ids: string[]) => {
    const previousWorkspaces = _get().workspaces;
    // 乐观更新：按新顺序重排
    const reordered = ids
      .map((id) => previousWorkspaces.find((ws) => ws.id === id))
      .filter((ws): ws is Workspace => ws !== undefined);
    set({ workspaces: reordered });

    try {
      await workspaceService.updateWorkspaceSortOrder(ids);
    } catch (err) {
      // 回滚
      set({ workspaces: previousWorkspaces });
      toast.error("Failed to update workspace order");
    }
  },
}));
