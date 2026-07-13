import { useCallback, useEffect } from "react";
import { useLoopEngineStore } from "@/stores/loop-engine";
import type { CreateLoopStateRequest } from "@/shared/types";

export function useLoopEngine(novelId: string | null) {
  const states = useLoopEngineStore((s) => s.states);
  const patterns = useLoopEngineStore((s) => s.patterns);
  const runLogs = useLoopEngineStore((s) => s.runLogs);
  const loading = useLoopEngineStore((s) => s.loading);
  const error = useLoopEngineStore((s) => s.error);
  const loadStates = useLoopEngineStore((s) => s.loadStates);
  const loadPatterns = useLoopEngineStore((s) => s.loadPatterns);
  const createState = useLoopEngineStore((s) => s.createState);
  const deleteState = useLoopEngineStore((s) => s.deleteState);
  const runCycle = useLoopEngineStore((s) => s.runCycle);
  const pauseLoop = useLoopEngineStore((s) => s.pauseLoop);
  const resumeLoop = useLoopEngineStore((s) => s.resumeLoop);
  const loadRunLogs = useLoopEngineStore((s) => s.loadRunLogs);

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
