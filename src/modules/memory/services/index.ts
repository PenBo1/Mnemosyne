// Memory 服务 —— 暴露 MemoryStore 的检索/统计/CRUD 能力给前端 hook。

import { ipc, ipcVoid } from "@/infrastructure/api";
import type { MemoryEntry, MemoryStats, CreateMemoryParams } from "@/shared/types/memory";

/** 列出某本书的全部 memory 条目 */
export async function listMemories(bookId: string): Promise<MemoryEntry[]> {
  return ipc<MemoryEntry[]>("memory_list", { bookId });
}

/** 按查询搜索 memory 条目（BM25 风格） */
export async function searchMemories(
  bookId: string,
  query: string,
  topK = 10,
): Promise<MemoryEntry[]> {
  return ipc<MemoryEntry[]>("memory_search", { bookId, query, topK });
}

/** 获取 memory 统计信息 */
export async function getMemoryStats(bookId: string): Promise<MemoryStats> {
  return ipc<MemoryStats>("memory_stats", { bookId });
}

/** 获取格式化的主上下文字符串（用于注入 prompt） */
export async function formatMemoryContext(bookId: string): Promise<string> {
  return ipc<string>("memory_format_context", { bookId });
}

/** 用户手动创建 memory 条目 */
export async function createMemory(
  params: CreateMemoryParams,
): Promise<MemoryEntry> {
  return ipc<MemoryEntry>("memory_create", {
    bookId: params.bookId,
    content: params.content,
    entryType: params.entryType,
    chapter: params.chapter ?? null,
    tags: params.tags ?? [],
  });
}

/** 更新已有的 memory 条目（content + tags） */
export async function updateMemory(
  bookId: string,
  entryId: string,
  content: string,
  tags: string[],
): Promise<MemoryEntry> {
  return ipc<MemoryEntry>("memory_update", {
    bookId,
    entryId,
    content,
    tags,
  });
}

/** 删除 memory 条目 */
export async function deleteMemory(
  bookId: string,
  entryId: string,
): Promise<boolean> {
  return ipcVoid("memory_delete", { bookId, entryId }).then(() => true);
}
