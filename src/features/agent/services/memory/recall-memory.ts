import { z } from "zod";
import { ipc } from "@/services/ipc";

export const RecallSearchSchema = z.object({
  query: z.string().min(1).max(500),
  limit: z.number().int().min(1).max(50).default(10),
});

export type RecallSearchParams = z.infer<typeof RecallSearchSchema>;

export interface RecallSearchResult {
  messageId: string;
  sessionId: string;
  role: "user" | "assistant";
  content: string;
  timestamp: string;
  relevanceScore: number;
}

export interface RecallMemoryToolDefinition {
  name: "recall_search";
  description: string;
  parameters: z.ZodType;
  execute: (args: unknown) => Promise<string>;
}

export const RECALL_SEARCH_DESC_KEY = "agentTools.recallSearchDesc";
export const DEFAULT_RECALL_SEARCH_DESC = "Search recent conversation history for relevant messages";

export function getRecallSearchTool(sessionId: string, description?: string): RecallMemoryToolDefinition {
  return {
    name: "recall_search",
    description: description ?? DEFAULT_RECALL_SEARCH_DESC,
    parameters: RecallSearchSchema,
    execute: async (args: unknown) => {
      const params = RecallSearchSchema.parse(args);
      const results = await ipc<RecallSearchResult[]>("agent_recall_search", {
        sessionId,
        query: params.query,
        limit: params.limit,
      });
      if (results.length === 0) {
        return "No relevant messages found in recent conversation history";
      }
      return results
        .map(
          (r) => `[${r.timestamp}] ${r.role === "user" ? "User" : "Assistant"}: ${r.content.slice(0, 500)}...`
        )
        .join("\n\n");
    },
  };
}

export async function recallSearch(sessionId: string, params: RecallSearchParams): Promise<RecallSearchResult[]> {
  const validated = RecallSearchSchema.parse(params);
  return ipc<RecallSearchResult[]>("agent_recall_search", {
    sessionId,
    query: validated.query,
    limit: validated.limit,
  });
}

// ── Panel/Session-oriented API（V2 命令，供 MemoryPanel UI 使用） ──

/** MemoryPanel 展示用的 recall 搜索结果条目（与组件原本地类型对齐）。 */
export interface RecallMemorySearchResult {
  id: string;
  role: string;
  content: string;
  createdAt: string;
}

/**
 * 搜索当前 session 的近期对话记忆（MemoryPanel 用）。
 * 封装 recall_memory_search IPC。
 */
export async function searchRecallMemory(
  sessionId: string,
  query: string,
): Promise<RecallMemorySearchResult[]> {
  return ipc<RecallMemorySearchResult[]>("recall_memory_search", { sessionId, query });
}