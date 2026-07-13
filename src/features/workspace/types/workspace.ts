// ── Workspace ──────────────────────────────────────────────

export interface Workspace {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
  /** 最近打开时间（用于恢复上次活动工作区） */
  last_opened_at: string | null;
}

export interface CreateWorkspaceRequest {
  name: string;
  path?: string;
}

/** Workspace store 接口状态（含 actions） */
export interface WorkspaceState {
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  loading: boolean;
  error: string | null;
  loadWorkspaces: () => Promise<void>;
  addWorkspace: (name: string, path?: string) => Promise<void>;
  removeWorkspace: (id: string) => Promise<void>;
  setActiveWorkspace: (id: string) => void;
}
