import { ipc } from "@/infrastructure/api";
import type { LlmCall, ToolExecution, TokenUsageStats, SandboxViolation } from "@/shared/types";

// ── API Functions ──────────────────────────────────────────

export async function getLlmCalls(sessionId: string, limit?: number): Promise<LlmCall[]> {
  return ipc<LlmCall[]>("ai_log_llm_calls", { sessionId, limit: limit ?? 50 });
}

export async function getToolExecutions(sessionId: string, limit?: number): Promise<ToolExecution[]> {
  return ipc<ToolExecution[]>("ai_log_tool_executions", { sessionId, limit: limit ?? 50 });
}

export async function getTokenUsage(sessionId: string): Promise<TokenUsageStats> {
  return ipc<TokenUsageStats>("ai_log_token_usage", { sessionId });
}

export async function getSandboxViolations(sessionId: string, limit?: number): Promise<SandboxViolation[]> {
  return ipc<SandboxViolation[]>("ai_log_sandbox_violations", { sessionId, limit: limit ?? 50 });
}
