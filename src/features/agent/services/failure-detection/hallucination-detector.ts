import { FailurePattern, type FailureReport, type AgentStep, type ToolRegistry } from "./types";

const DEFAULT_TOOLS = new Set([
  "fs_read_file",
  "fs_write_file",
  "fs_list_dir",
  "fs_create_dir",
  "fs_delete",
  "fs_move",
  "fs_copy",
  "fs_exists",
  "fs_read_multiple_files",
  "shell_exec",
  "http_request",
  "core_memory_append",
  "core_memory_replace",
  "recall_search",
  "archival_insert",
  "archival_search",
  "todo_write",
  "todo_read",
  "plan_diff_review",
  "ask_followup_question",
  "attempt_completion",
  "search_files",
  "list_files",
  "read_file",
  "write_to_file",
  "execute_command",
  "browser_search",
  "browser_click",
  "browser_scroll",
]);

export class HallucinationDetector {
  private toolRegistry: ToolRegistry | null = null;
  private knownTools: Set<string>;

  constructor(toolRegistry?: ToolRegistry) {
    this.toolRegistry = toolRegistry ?? null;
    this.knownTools = new Set(DEFAULT_TOOLS);
  }

  registerTool(name: string): void {
    this.knownTools.add(name);
  }

  registerTools(names: string[]): void {
    for (const name of names) {
      this.knownTools.add(name);
    }
  }

  isValidTool(name: string): boolean {
    if (this.toolRegistry) {
      return this.toolRegistry.hasTool(name);
    }
    return this.knownTools.has(name);
  }

  detect(step: AgentStep): FailureReport | null {
    if (step.type !== "tool_call" || !step.toolName) {
      return null;
    }

    if (!this.isValidTool(step.toolName)) {
      const availableTools = this.getAvailableTools();
      return {
        pattern: FailurePattern.HallucinatedAction,
        severity: "error",
        message: `Agent attempted to call non-existent tool: ${step.toolName}`,
        suggestion: `Available tools include: ${availableTools.slice(0, 5).join(", ")} and ${availableTools.length - 5} more. Please verify the tool name.`,
        timestamp: Date.now(),
        metadata: {
          invalidToolName: step.toolName,
          availableTools,
          i18nKey: "failureDetection.hallucinatedTool",
          i18nParams: { tool: step.toolName },
        },
      };
    }

    return null;
  }

  private getAvailableTools(): string[] {
    if (this.toolRegistry) {
      return this.toolRegistry.getToolNames();
    }
    return Array.from(this.knownTools);
  }
}