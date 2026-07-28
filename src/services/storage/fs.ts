/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 文件系统服务 - 提供主机文件读取与目录列表功能
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { ipc } from "@/services/ipc";
import type { FileEntry } from "@/types";

// ── 文件操作 ────────────────────────────────────────────────────────────────

/**
 * 读取主机上的文件内容
 */
export async function readFile(path: string): Promise<string> {
  return ipc<string>("fs_read_file", { path });
}

/**
 * 列出主机目录下的文件和子目录
 */
export async function listDirectory(path: string): Promise<FileEntry[]> {
  return ipc<FileEntry[]>("fs_list_directory", { path });
}