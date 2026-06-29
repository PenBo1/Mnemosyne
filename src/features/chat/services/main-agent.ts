import { ipc } from "@/infrastructure/api";
import type { MainAgentEvent } from "@/shared/types/main-agent";

export async function executeGoal(params: {
  sessionId: string;
  goal: string;
}): Promise<string> {
  return ipc<string>("main_agent_execute", params);
}

export async function respondToConfirmation(params: {
  sessionId: string;
  approved: boolean;
  modifiedArgs?: string;
}): Promise<string> {
  return ipc<string>("main_agent_respond", params);
}

export async function listSessions(): Promise<string[]> {
  return ipc<string[]>("main_agent_list_sessions");
}

export async function cancelExecution(sessionId: string): Promise<string> {
  return ipc<string>("main_agent_cancel", { sessionId });
}

export function listenToMainAgentEvents(
  callback: (event: MainAgentEvent) => void
): () => void {
  let unlistenFn: (() => void) | null = null;
  import("@tauri-apps/api/event").then(({ listen }) => {
    listen<MainAgentEvent>("main-agent:progress", (e) => {
      callback(e.payload);
    }).then((fn) => {
      unlistenFn = fn;
    });
  });
  return () => {
    unlistenFn?.();
  };
}
