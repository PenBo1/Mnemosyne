import { open } from "@tauri-apps/plugin-dialog";
import { ipc } from "@/infrastructure/api";
import type { Workspace, CreateWorkspaceRequest } from "@/shared/types";

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

export async function createWorkspace(req: CreateWorkspaceRequest): Promise<Workspace> {
  return ipc<Workspace>("create_workspace", { req });
}

export async function deleteWorkspace(id: string): Promise<boolean> {
  return ipc<boolean>("delete_workspace", { id });
}
