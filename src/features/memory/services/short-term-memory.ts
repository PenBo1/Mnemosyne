// 短期记忆服务 —— 暴露 memory_short_term 表的查询能力给前端 hook。
//
// 后端命令:
// - short_term_memory_list_by_date: 按日期查询所有 session 摘要
// - short_term_memory_for_session: 按 session_id 获取最新摘要
// - short_term_memory_list_by_book: 按 book_id 列出该书的 session 摘要
// - short_term_memory_list_by_range: 按日期范围列出
// - short_term_memory_regenerate: 主动重新生成 session 摘要
// - short_term_memory_stats: 统计信息

import { ipc, ipcVoid } from "@/services/ipc";
import type { ShortTermMemoryRow, ShortTermMemoryStats } from "@/features/memory/types/short-term-memory";

/** 按日期查询所有 session 摘要(用于每日回顾 UI) */
export async function listShortTermMemoryByDate(entryDate: string): Promise<ShortTermMemoryRow[]> {
  return ipc<ShortTermMemoryRow[]>("short_term_memory_list_by_date", { entryDate });
}

/** 按 session_id 获取最新摘要(用于 session 恢复时回填"上次进展") */
export async function getShortTermMemoryForSession(sessionId: string): Promise<ShortTermMemoryRow | null> {
  return ipc<ShortTermMemoryRow | null>("short_term_memory_for_session", { sessionId });
}

/** 按 book_id 列出该书的 session 摘要(用于小说创作的上下文回顾) */
export async function listShortTermMemoryByBook(
  bookId: string,
  limit = 50,
): Promise<ShortTermMemoryRow[]> {
  return ipc<ShortTermMemoryRow[]>("short_term_memory_list_by_book", { bookId, limit });
}

/** 按日期范围列出短期记忆(用于"最近 N 天"或自定义范围) */
export async function listShortTermMemoryByRange(
  startDate: string,
  endDate: string,
): Promise<ShortTermMemoryRow[]> {
  return ipc<ShortTermMemoryRow[]>("short_term_memory_list_by_range", { startDate, endDate });
}

/** 主动为 session 重新生成摘要(用户点击"重新生成"时触发) */
export async function regenerateShortTermMemory(
  sessionId: string,
  bookId?: string | null,
  agentRole?: string | null,
): Promise<boolean> {
  return ipcVoid("short_term_memory_regenerate", {
    sessionId,
    bookId: bookId ?? null,
    agentRole: agentRole ?? null,
  }).then(() => true);
}

/** 获取短期记忆统计 */
export async function getShortTermMemoryStats(): Promise<ShortTermMemoryStats> {
  return ipc<ShortTermMemoryStats>("short_term_memory_stats", {});
}
