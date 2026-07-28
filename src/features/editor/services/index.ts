// 编辑器 IPC 服务封装
import { ipc } from "@/services/ipc";
import type { EditorFileContent, RecentFile } from "@/features/editor/types";

/** 读取文件内容（经 Security Kernel 校验） */
export async function editorReadFile(
  path: string,
  workspaceId?: string,
): Promise<EditorFileContent> {
  return ipc<EditorFileContent>("editor_read_file", {
    path,
    workspaceId: workspaceId ?? null,
  });
}

/** 原子写入文件内容（经 Security Kernel 校验） */
export async function editorWriteFile(
  path: string,
  content: string,
  workspaceId?: string,
): Promise<number> {
  return ipc<number>("editor_write_file", {
    path,
    content,
    workspaceId: workspaceId ?? null,
  });
}

/** 列出最近编辑文件（当前为 stub，返回空数组） */
export async function editorListRecent(limit = 20): Promise<RecentFile[]> {
  return ipc<RecentFile[]>("editor_list_recent", { limit });
}
