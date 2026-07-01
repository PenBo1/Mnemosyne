import { ipc, ipcVoid } from "@/infrastructure/api";
import type { Agent, Message, SendMessageParams, AttachmentSpec } from "@/shared/types";
import type { AiModelConfig } from "@/shared/settings";

// re-export 让 useChat 等调用方可通过 agentService.AttachmentSpec 引用
export type { AttachmentSpec };

export async function fetchAgents(): Promise<Agent[]> {
  return ipc<Agent[]>("list_agents");
}

export async function updateAgent(id: string, updates: Partial<Agent>): Promise<Agent> {
  return ipc<Agent>("update_agent", { req: { id, ...updates } });
}

export async function toggleAgentStatus(id: string): Promise<Agent> {
  return ipc<Agent>("toggle_agent_status", { id });
}

/**
 * 列出用户配置的所有 AI 模型（S9: per-agent 模型路由用）。
 * 返回 AiModelConfig 列表，前端选中后将 id 作为 update_agent 的 model 字段传回。
 */
export async function fetchAiModels(): Promise<AiModelConfig[]> {
  return ipc<AiModelConfig[]>("list_ai_models");
}

export async function sendMessage(params: SendMessageParams): Promise<void> {
  await ipcVoid("agent_send_message", { ...params });
}

export async function approveTool(toolCallId: string, approved: boolean): Promise<void> {
  await ipc<void>("agent_approve_tool", { toolCallId, approved });
}

/**
 * 响应 SafetyGate 的确认请求（替代已废弃的 approveTool）。
 *
 * - approved=true + autoApproveSimilar=true：批准 + 后续同名工具自动通过
 * - approved=true + autoApproveSimilar=false：仅批准本次
 * - modifiedArgs 提供时：用修改后的参数执行（approved 自动设为 true）
 * - 否则：拒绝
 */
export async function respondConfirmation(params: {
  sessionId: string;
  toolCallId: string;
  approved: boolean;
  autoApproveSimilar: boolean;
  modifiedArgs?: string;
}): Promise<void> {
  await ipcVoid("agent_respond_confirmation", {
    sessionId: params.sessionId,
    toolCallId: params.toolCallId,
    approved: params.approved,
    autoApproveSimilar: params.autoApproveSimilar,
    modifiedArgs: params.modifiedArgs,
  });
}

export async function cancelAgent(sessionId: string): Promise<void> {
  await ipc<void>("agent_cancel", { sessionId });
}

export async function compactSession(sessionId: string): Promise<void> {
  await ipc<void>("agent_compact", { sessionId });
}

export async function restartAgent(): Promise<void> {
  await ipc<void>("agent_restart");
}

export async function fetchMessages(sessionId: string): Promise<Message[]> {
  return ipc<Message[]>("agent_messages", { sessionId });
}
