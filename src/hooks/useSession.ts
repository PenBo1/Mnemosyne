import { useCallback, useEffect } from "react";
import { useAgentStore } from "@/stores/agent";

export function useSession(novelId?: string) {
  const {
    sessions,
    currentSessionId,
    loading,
    error,
    loadSessions,
    createSession,
    switchSession,
    deleteSession,
  } = useAgentStore();

  useEffect(() => {
    loadSessions(novelId);
  }, [novelId, loadSessions]);

  const currentSession = sessions.find((s) => s.id === currentSessionId) || null;

  const create = useCallback(
    async (title?: string) => {
      return createSession(novelId, title);
    },
    [novelId, createSession]
  );

  return {
    sessions,
    currentSession,
    currentSessionId,
    loading,
    error,
    create,
    switch: switchSession,
    remove: deleteSession,
  };
}
