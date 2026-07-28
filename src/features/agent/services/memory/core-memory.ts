import { z } from "zod";
import { ipc, ipcVoid } from "@/services/ipc";

export const CoreMemoryAppendSchema = z.object({
  section: z.string().min(1).max(100),
  content: z.string().min(1).max(50000),
});

export const CoreMemoryReplaceSchema = z.object({
  section: z.string().min(1).max(100),
  oldContent: z.string().min(1),
  newContent: z.string().min(1).max(50000),
});

export type CoreMemoryAppendParams = z.infer<typeof CoreMemoryAppendSchema>;
export type CoreMemoryReplaceParams = z.infer<typeof CoreMemoryReplaceSchema>;

export interface CoreMemoryToolDefinition {
  name: "core_memory_append" | "core_memory_replace";
  description: string;
  parameters: z.ZodType;
  execute: (args: unknown) => Promise<string>;
}

export const CORE_MEMORY_APPEND_DESC_KEY = "agentTools.coreMemoryAppendDesc";
export const CORE_MEMORY_REPLACE_DESC_KEY = "agentTools.coreMemoryReplaceDesc";

export const DEFAULT_CORE_MEMORY_APPEND_DESC = "Append content to a section in MEMORY.md";
export const DEFAULT_CORE_MEMORY_REPLACE_DESC = "Replace content in a MEMORY.md section";

export function getCoreMemoryAppendTool(role: string, description?: string): CoreMemoryToolDefinition {
  return {
    name: "core_memory_append",
    description: description ?? DEFAULT_CORE_MEMORY_APPEND_DESC,
    parameters: CoreMemoryAppendSchema,
    execute: async (args: unknown) => {
      const params = CoreMemoryAppendSchema.parse(args);
      await ipcVoid("agent_memory_append", {
        role,
        section: params.section,
        content: params.content,
      });
      return `Successfully appended to section "${params.section}" in MEMORY.md`;
    },
  };
}

export function getCoreMemoryReplaceTool(role: string, description?: string): CoreMemoryToolDefinition {
  return {
    name: "core_memory_replace",
    description: description ?? DEFAULT_CORE_MEMORY_REPLACE_DESC,
    parameters: CoreMemoryReplaceSchema,
    execute: async (args: unknown) => {
      const params = CoreMemoryReplaceSchema.parse(args);
      const replaced = await ipc<boolean>("agent_memory_replace", {
        role,
        section: params.section,
        oldContent: params.oldContent,
        newContent: params.newContent,
      });
      if (!replaced) {
        throw new Error("Content to replace not found in section");
      }
      return `Successfully replaced content in section "${params.section}"`;
    },
  };
}

export async function coreMemoryAppend(role: string, params: CoreMemoryAppendParams): Promise<void> {
  const validated = CoreMemoryAppendSchema.parse(params);
  await ipcVoid("agent_memory_append", {
    role,
    section: validated.section,
    content: validated.content,
  });
}

export async function coreMemoryReplace(role: string, params: CoreMemoryReplaceParams): Promise<boolean> {
  const validated = CoreMemoryReplaceSchema.parse(params);
  return ipc<boolean>("agent_memory_replace", {
    role,
    section: validated.section,
    oldContent: validated.oldContent,
    newContent: validated.newContent,
  });
}

// ── Panel/Session-oriented API（V2 命令，供 MemoryPanel UI 使用） ──

/**
 * 追加内容到当前 session 的核心记忆（MemoryPanel 用）。
 * 封装 core_memory_append IPC，参数语义与组件原直调保持一致。
 */
export async function appendCoreMemory(
  sessionId: string,
  content: string,
  section?: string,
): Promise<void> {
  await ipcVoid("core_memory_append", {
    sessionId,
    content,
    section: section ?? undefined,
  });
}

/**
 * 替换当前 session 的核心记忆内容（MemoryPanel 用）。
 * 封装 core_memory_replace IPC。
 */
export async function replaceCoreMemory(
  sessionId: string,
  content: string,
  section?: string,
): Promise<void> {
  await ipcVoid("core_memory_replace", {
    sessionId,
    content,
    section: section ?? undefined,
  });
}