// 项目记忆服务 —— 暴露 workspace 级 project_memory.md 的读写能力给前端。
//
// 后端命令:
// - project_memory_get(workspace_id) → 当前内容(不存在返回空串)
// - project_memory_update(workspace_id, content) → 全量覆盖
// - project_memory_append(workspace_id, section) → 追加段落
// - project_memory_clear(workspace_id) → 清空内容(保留文件)
// - project_memory_delete(workspace_id) → 删除文件 + 目录
// - project_memory_stats(workspace_id) → 字节数/字符数/上限/存在性

import { ipc, ipcVoid } from "@/services/ipc";

export interface ProjectMemoryStats {
  bytes: number;
  chars: number;
  maxBytes: number;
  exists: boolean;
}

/** 读取 workspace 的 project_memory.md 内容(不存在返回空字符串) */
export function getProjectMemory(workspaceId: string): Promise<string> {
  return ipc<string>("project_memory_get", { workspaceId });
}

/** 全量覆盖 workspace 的 project_memory.md */
export async function updateProjectMemory(
  workspaceId: string,
  content: string,
): Promise<void> {
  await ipcVoid("project_memory_update", { workspaceId, content });
}

/** 在文件末尾追加一段内容(自动加换行分隔) */
export async function appendProjectMemory(
  workspaceId: string,
  section: string,
): Promise<void> {
  await ipcVoid("project_memory_append", { workspaceId, section });
}

/** 清空内容(保留文件路径) */
export async function clearProjectMemory(workspaceId: string): Promise<void> {
  await ipcVoid("project_memory_clear", { workspaceId });
}

/** 删除文件 + 目录(通常由 delete_workspace 自动调用) */
export async function deleteProjectMemory(workspaceId: string): Promise<void> {
  await ipcVoid("project_memory_delete", { workspaceId });
}

/** 获取字节数 / 字符数 / 上限 / 存在性 */
export function getProjectMemoryStats(workspaceId: string): Promise<ProjectMemoryStats> {
  return ipc<ProjectMemoryStats>("project_memory_stats", { workspaceId });
}
