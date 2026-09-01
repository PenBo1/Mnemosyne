import { open } from "@tauri-apps/plugin-dialog";
import { ipc, ipcVoid } from "@/services/ipc";
import type { Workspace, CreateWorkspaceRequest } from "@/features/workspace/types";

export async function pickDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select workspace directory",
  });
  if (typeof selected === "string") {
    return selected;
  }
  return null;
}

export async function fetchWorkspaces(): Promise<Workspace[]> {
  return ipc<Workspace[]>("list_workspaces");
}

export async function fetchArchivedWorkspaces(): Promise<Workspace[]> {
  return ipc<Workspace[]>("list_archived_workspaces");
}

export async function createWorkspace(req: CreateWorkspaceRequest): Promise<Workspace> {
  return ipc<Workspace>("create_workspace", { req });
}

export async function deleteWorkspace(id: string): Promise<boolean> {
  return ipc<boolean>("delete_workspace", { id });
}

/** 更新工作区最近打开时间（用于恢复上次活动工作区） */
export async function touchWorkspace(id: string): Promise<void> {
  return ipcVoid("touch_workspace", { id });
}

/** 归档工作区 */
export async function archiveWorkspace(id: string): Promise<Workspace> {
  return ipc<Workspace>("archive_workspace", { id });
}

/** 恢复工作区 */
export async function restoreWorkspace(id: string): Promise<Workspace> {
  return ipc<Workspace>("restore_workspace", { id });
}

/** 更新工作区排序顺序 */
export async function updateWorkspaceSortOrder(ids: string[]): Promise<void> {
  return ipcVoid("update_workspace_sort_order", { ids });
}
