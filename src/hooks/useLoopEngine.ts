import { useCallback, useEffect } from "react";
import { useLoopEngineStore } from "@/stores/loop-engine";
import type { CreateLoopStateRequest } from "@/types";

export function useLoopEngine(novelId: string | null) {
  const {
    states,
    patterns,
    runLogs,
    loading,
    error,
    loadStates,
    loadPatterns,
    createState,
    deleteState,
    runCycle,
    pauseLoop,
    resumeLoop,
    loadRunLogs,
  } = useLoopEngineStore();

  useEffect(() => {
    loadPatterns();
  }, [loadPatterns]);

  useEffect(() => {
    if (novelId) {
      loadStates(novelId);
    }
  }, [novelId, loadStates]);

  const handleCreate = useCallback(
    async (req: CreateLoopStateRequest) => {
      if (!novelId) return;
      return createState(novelId, req);
    },
    [novelId, createState]
  );

  const handleRun = useCallback(
    async (stateId: string) => {
      const log = await runCycle(stateId);
      if (novelId) {
        await loadRunLogs(stateId);
      }
      return log;
    },
    [runCycle, loadRunLogs, novelId]
  );

  return {
    states,
    patterns,
    runLogs,
    loading,
    error,
    createState: handleCreate,
    deleteState,
    runCycle: handleRun,
    pauseLoop,
    resumeLoop,
    loadRunLogs,
    reload: () => {
      if (novelId) loadStates(novelId);
    },
  };
}
