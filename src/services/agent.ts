import { ipc } from "@/lib/ipc";
import type { Agent, Message, AgentIdentity } from "@/types";

export async function fetchAgents(): Promise<Agent[]> {
  return ipc<Agent[]>("list_agents");
}

export async function updateAgent(id: string, updates: Partial<Agent>): Promise<Agent> {
  return ipc<Agent>("update_agent", { req: { id, ...updates } });
}

export async function toggleAgentStatus(id: string): Promise<Agent> {
  return ipc<Agent>("toggle_agent_status", { id });
}

export async function getAgentIdentity(role: string): Promise<AgentIdentity> {
  return ipc<AgentIdentity>("get_agent_identity", { role });
}

export async function updateAgentIdentity(
  role: string,
  updates: { soul?: string; context?: string; memory?: string }
): Promise<AgentIdentity> {
  return ipc<AgentIdentity>("update_agent_identity", { req: { role, ...updates } });
}

export async function sendMessage(params: { sessionId: string; content: string }): Promise<void> {
  await ipc<void>("agent_send_message", params);
}

export async function approveTool(toolCallId: string, approved: boolean): Promise<void> {
  await ipc<void>("agent_approve_tool", { toolCallId, approved });
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
