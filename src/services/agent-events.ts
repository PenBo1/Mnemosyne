import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AgentEvent } from "@/types";

export async function onAgentEvent(
  sessionId: string,
  callback: (event: AgentEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentEvent>("agent-event", (event) => {
    if (event.payload.session_id === sessionId) {
      callback(event.payload);
    }
  });
}
