import type { CoreMemoryToolDefinition } from "./core-memory";
import type { RecallMemoryToolDefinition } from "./recall-memory";
import type { ArchivalMemoryToolDefinition } from "./archival-memory";

import {
  CoreMemoryAppendSchema,
  CoreMemoryReplaceSchema,
  getCoreMemoryAppendTool,
  getCoreMemoryReplaceTool,
  coreMemoryAppend,
  coreMemoryReplace,
  CORE_MEMORY_APPEND_DESC_KEY,
  CORE_MEMORY_REPLACE_DESC_KEY,
  DEFAULT_CORE_MEMORY_APPEND_DESC,
  DEFAULT_CORE_MEMORY_REPLACE_DESC,
  type CoreMemoryAppendParams,
  type CoreMemoryReplaceParams,
} from "./core-memory";

import {
  RecallSearchSchema,
  getRecallSearchTool,
  recallSearch,
  RECALL_SEARCH_DESC_KEY,
  DEFAULT_RECALL_SEARCH_DESC,
  type RecallSearchParams,
  type RecallSearchResult,
} from "./recall-memory";

import {
  ArchivalInsertSchema,
  ArchivalSearchSchema,
  getArchivalInsertTool,
  getArchivalSearchTool,
  archivalInsert,
  archivalSearch,
  ARCHIVAL_INSERT_DESC_KEY,
  ARCHIVAL_SEARCH_DESC_KEY,
  DEFAULT_ARCHIVAL_INSERT_DESC,
  DEFAULT_ARCHIVAL_SEARCH_DESC,
  type ArchivalInsertParams,
  type ArchivalSearchParams,
  type ArchivalEntry,
} from "./archival-memory";

export {
  CoreMemoryAppendSchema,
  CoreMemoryReplaceSchema,
  type CoreMemoryAppendParams,
  type CoreMemoryReplaceParams,
  getCoreMemoryAppendTool,
  getCoreMemoryReplaceTool,
  coreMemoryAppend,
  coreMemoryReplace,
  CORE_MEMORY_APPEND_DESC_KEY,
  CORE_MEMORY_REPLACE_DESC_KEY,
  DEFAULT_CORE_MEMORY_APPEND_DESC,
  DEFAULT_CORE_MEMORY_REPLACE_DESC,
};

export {
  RecallSearchSchema,
  type RecallSearchParams,
  type RecallSearchResult,
  getRecallSearchTool,
  recallSearch,
  RECALL_SEARCH_DESC_KEY,
  DEFAULT_RECALL_SEARCH_DESC,
};

export {
  ArchivalInsertSchema,
  ArchivalSearchSchema,
  type ArchivalInsertParams,
  type ArchivalSearchParams,
  type ArchivalEntry,
  getArchivalInsertTool,
  getArchivalSearchTool,
  archivalInsert,
  archivalSearch,
  ARCHIVAL_INSERT_DESC_KEY,
  ARCHIVAL_SEARCH_DESC_KEY,
  DEFAULT_ARCHIVAL_INSERT_DESC,
  DEFAULT_ARCHIVAL_SEARCH_DESC,
};

export type MemoryToolDefinition =
  | CoreMemoryToolDefinition
  | RecallMemoryToolDefinition
  | ArchivalMemoryToolDefinition;

export interface MemoryToolContext {
  role: string;
  sessionId: string;
}

export function getMemoryTools(ctx: MemoryToolContext): MemoryToolDefinition[] {
  return [
    getCoreMemoryAppendTool(ctx.role),
    getCoreMemoryReplaceTool(ctx.role),
    getRecallSearchTool(ctx.sessionId),
    getArchivalInsertTool(ctx.role),
    getArchivalSearchTool(ctx.role),
  ];
}

export function getMemoryToolSchemas(): Record<string, unknown> {
  return {
    core_memory_append: {
      type: "object",
      properties: {
        section: { type: "string", minLength: 1, maxLength: 100 },
        content: { type: "string", minLength: 1, maxLength: 50000 },
      },
      required: ["section", "content"],
    },
    core_memory_replace: {
      type: "object",
      properties: {
        section: { type: "string", minLength: 1, maxLength: 100 },
        oldContent: { type: "string", minLength: 1 },
        newContent: { type: "string", minLength: 1, maxLength: 50000 },
      },
      required: ["section", "oldContent", "newContent"],
    },
    recall_search: {
      type: "object",
      properties: {
        query: { type: "string", minLength: 1, maxLength: 500 },
        limit: { type: "integer", minimum: 1, maximum: 50, default: 10 },
      },
      required: ["query"],
    },
    archival_insert: {
      type: "object",
      properties: {
        content: { type: "string", minLength: 1, maxLength: 100000 },
        tags: { type: "array", items: { type: "string", maxLength: 50 }, maxItems: 20 },
        citation: {
          type: "object",
          properties: {
            source: { type: "string", maxLength: 200 },
            url: { type: "string", format: "uri" },
            timestamp: { type: "string", format: "date-time" },
          },
        },
      },
      required: ["content"],
    },
    archival_search: {
      type: "object",
      properties: {
        query: { type: "string", minLength: 1, maxLength: 500 },
        tags: { type: "array", items: { type: "string", maxLength: 50 }, maxItems: 10 },
        limit: { type: "integer", minimum: 1, maximum: 50, default: 10 },
      },
      required: ["query"],
    },
  };
}