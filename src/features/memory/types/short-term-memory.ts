// 短期记忆类型 —— 对应 Rust 的 ShortTermMemoryRow
//
// 用 SQLite 持久化,JSON 数组形式存储 key_topics
//
// 字段说明:
// - entryDate: YYYY-MM-DD(按日聚合)
// - keyTopics: JSON 字符串数组(从 LLM 摘要中提取)
// - agentRole: 角色名(main/auditor/reviser)
// - tokenCount / messageCount: 本次 session 的用量指标

export interface ShortTermMemoryRow {
  id: string;
  sessionId: string;
  bookId: string | null;
  /** YYYY-MM-DD */
  entryDate: string;
  summary: string;
  /** JSON 字符串,如 `["chapter-3","dialogue"]`,前端需 JSON.parse */
  keyTopics: string;
  agentRole: string | null;
  tokenCount: number;
  messageCount: number;
  createdAt: string;
}

export interface ShortTermMemoryStats {
  total: number;
  todayCount: number;
  last7DaysCount: number;
}

/** 解析 keyTopics JSON 字符串为字符串数组(失败返回空数组) */
export function parseKeyTopics(raw: string): string[] {
  try {
    const arr = JSON.parse(raw);
    if (Array.isArray(arr)) {
      return arr.filter((x): x is string => typeof x === "string");
    }
  } catch {
    // ignore parse error, return empty
  }
  return [];
}
