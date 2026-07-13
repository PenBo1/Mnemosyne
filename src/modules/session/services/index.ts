import { ipc } from "@/infrastructure/api";
import type { Session, Message } from "@/shared/types";

export async function createSession(
  novelId?: string,
  title?: string,
  workspaceId?: string
): Promise<Session> {
  return ipc<Session>("session_create", { novelId, workspaceId, title });
}

export async function listSessions(novelId?: string): Promise<Session[]> {
  return ipc<Session[]>("session_list", { novelId });
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
