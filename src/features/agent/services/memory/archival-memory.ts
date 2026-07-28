import { z } from "zod";
import { ipc } from "@/services/ipc";

export const ArchivalInsertSchema = z.object({
  content: z.string().min(1).max(100000),
  tags: z.array(z.string().max(50)).max(20).optional().default([]),
  citation: z
    .object({
      source: z.string().max(200).optional(),
      url: z.string().url().optional(),
      timestamp: z.string().datetime().optional(),
    })
    .optional(),
});

export const ArchivalSearchSchema = z.object({
  query: z.string().min(1).max(500),
  tags: z.array(z.string().max(50)).max(10).optional(),
  limit: z.number().int().min(1).max(50).default(10),
});

export type ArchivalInsertParams = z.infer<typeof ArchivalInsertSchema>;
export type ArchivalSearchParams = z.infer<typeof ArchivalSearchSchema>;

export interface ArchivalEntry {
  id: number;
  role: string;
  content: string;
  tags: string[];
  citationSource?: string;
  citationUrl?: string;
  citationTimestamp?: string;
  createdAt: string;
}

export interface ArchivalMemoryToolDefinition {
  name: "archival_insert" | "archival_search";
  description: string;
  parameters: z.ZodType;
  execute: (args: unknown) => Promise<string>;
}

export const ARCHIVAL_INSERT_DESC_KEY = "agentTools.archivalInsertDesc";
export const ARCHIVAL_SEARCH_DESC_KEY = "agentTools.archivalSearchDesc";

export const DEFAULT_ARCHIVAL_INSERT_DESC = "Insert long-term memory into archival storage";
export const DEFAULT_ARCHIVAL_SEARCH_DESC = "Search the archival memory store";

export function getArchivalInsertTool(role: string, description?: string): ArchivalMemoryToolDefinition {
  return {
    name: "archival_insert",
    description: description ?? DEFAULT_ARCHIVAL_INSERT_DESC,
    parameters: ArchivalInsertSchema,
    execute: async (args: unknown) => {
      const params = ArchivalInsertSchema.parse(args);
      const entry = await ipc<ArchivalEntry>("agent_archival_insert", {
        role,
        content: params.content,
        tags: params.tags,
        citation: params.citation ?? null,
      });
      return `Successfully inserted archival memory entry with id=${entry.id}`;
    },
  };
}

export function getArchivalSearchTool(role: string, description?: string): ArchivalMemoryToolDefinition {
  return {
    name: "archival_search",
    description: description ?? DEFAULT_ARCHIVAL_SEARCH_DESC,
    parameters: ArchivalSearchSchema,
    execute: async (args: unknown) => {
      const params = ArchivalSearchSchema.parse(args);
      const results = await ipc<ArchivalEntry[]>("agent_archival_search", {
        role,
        query: params.query,
        tags: params.tags ?? [],
        limit: params.limit,
      });
      if (results.length === 0) {
        return "No archival memories found matching the query";
      }
      return results
        .map(
          (entry) =>
            `[${entry.createdAt}] ${entry.content.slice(0, 500)}... (tags: ${entry.tags.join(", ") || "none"})`
        )
        .join("\n\n");
    },
  };
}

export async function archivalInsert(role: string, params: ArchivalInsertParams): Promise<ArchivalEntry> {
  const validated = ArchivalInsertSchema.parse(params);
  return ipc<ArchivalEntry>("agent_archival_insert", {
    role,
    content: validated.content,
    tags: validated.tags,
    citation: validated.citation ?? null,
  });
}

export async function archivalSearch(role: string, params: ArchivalSearchParams): Promise<ArchivalEntry[]> {
  const validated = ArchivalSearchSchema.parse(params);
  return ipc<ArchivalEntry[]>("agent_archival_search", {
    role,
    query: validated.query,
    tags: validated.tags ?? [],
    limit: validated.limit,
  });
}

// ── Panel/Session-oriented API（V2 命令，供 MemoryPanel UI 使用） ──

/** MemoryPanel 展示用的 archival 搜索结果条目（与组件原本地类型对齐）。 */
export interface ArchivalMemorySearchResult {
  id: string;
  content: string;
  similarity: number;
  createdAt: string;
  metadata?: Record<string, unknown>;
}

/**
 * 搜索长期归档记忆（MemoryPanel 用）。
 * 封装 archival_memory_search IPC。
 */
export async function searchArchivalMemory(
  sessionId: string,
  query: string,
): Promise<ArchivalMemorySearchResult[]> {
  return ipc<ArchivalMemorySearchResult[]>("archival_memory_search", { sessionId, query });
}