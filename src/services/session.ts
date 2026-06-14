import { ipc } from "@/lib/ipc";
import type { Session, Message } from "@/types";

export async function createSession(novelId?: string, title?: string): Promise<Session> {
  return ipc<Session>("session_create", { novelId, title });
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
