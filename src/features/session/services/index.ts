import { ipc } from "@/services/ipc";
import type { Session, Message } from "@/types/session";

export async function createSession(
  novelId?: string,
  title?: string,
  workspaceId?: string
): Promise<Session> {
  return ipc<Session>("session_create", { novelId, workspaceId, title });
}

export async function listSessions(
  novelId?: string,
  workspaceId?: string,
): Promise<Session[]> {
  return ipc<Session[]>("session_list", { novelId, workspaceId });
}

export async function getSession(id: string): Promise<Session> {
  return ipc<Session>("session_get", { id });
}

export async function deleteSession(id: string): Promise<boolean> {
  return ipc<boolean>("session_delete", { id });
}

export async function listMessages(sessionId: string): Promise<Message[]> {
  return ipc<Message[]>("session_messages", { sessionId });
}

/** assistant 消息的元数据（记录 thinking/model/token，对齐后端 MessageMetaInput）。 */
export interface MessageMetaInput {
  thinkingContent?: string;
  model?: string;
  provider?: string;
  inputTokens?: number;
  outputTokens?: number;
  latencyMs?: number;
}

/**
 * 写入一条消息到数据库（user / assistant / system / tool）。
 *
 * - `meta` 仅 assistant 回复需要，记录 thinking/model/token/latency
 * - 其他角色传 `undefined` 即可
 */
export async function createMessage(
  sessionId: string,
  role: "user" | "assistant" | "system" | "tool",
  content: string,
  meta?: MessageMetaInput,
  toolCalls?: string,
  toolResults?: string,
): Promise<Message> {
  return ipc<Message>("message_create", {
    sessionId,
    role,
    content,
    toolCalls: toolCalls ?? null,
    toolResults: toolResults ?? null,
    meta: meta ?? null,
  });
}
