// ── Memory ─────────────────────────────────────────────────
//
// 对齐后端 `domain/agents/base.rs` 的 `MemoryEntry` / `MemoryType`。
// 前端旧版 `Memory` 接口（title/created_at/updated_at）已废弃 ——
// 后端 MemoryStore 是 Agent 跨章节持久化的事实条目，无 title 字段，
// 用 content 首行或前若干字符作为展示标题即可。

/// Memory 条目类型（对齐后端 MemoryType 枚举的 snake_case 字符串表示）
export type MemoryType =
  | "character"
  | "plot"
  | "setting"
  | "dialogue"
  | "fact"
  | "style";

/// Memory 条目（对齐后端 MemoryEntry）
export interface MemoryEntry {
  id: string;
  content: string;
  entry_type: MemoryType;
  chapter: number | null;
  timestamp: string;
  tags: string[];
}

/// Memory 统计信息（对齐后端 MemoryStats）
export interface MemoryStats {
  /// 主上下文条目数
  main: number;
  /// 归档存储条目数
  archival: number;
}

/// 全部 memory 类型（用于 UI 下拉选择）
export const MEMORY_TYPES: MemoryType[] = [
  "character",
  "plot",
  "setting",
  "dialogue",
  "fact",
  "style",
];

/** 创建 memory 条目的参数 */
export interface CreateMemoryParams {
  bookId: string;
  content: string;
  entryType: MemoryType;
  chapter?: number | null;
  tags?: string[];
}
