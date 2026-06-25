import { create } from "zustand";
import type {
  LoopState,
  LoopEngineState,
  CreateLoopStateRequest,
  LoopConfig,
} from "@/types";
import * as loopService from "@/services/loop-engine";
import { toast } from "sonner";

export const useLoopEngineStore = create<LoopEngineState>((set, _get) => ({
  states: [],
  patterns: [],
  runLogs: [],
  loading: false,
  error: null,

  loadStates: async (novelId: string) => {
    set({ loading: true, error: null });
    try {
      const states = await loopService.getLoopStates(novelId);
      set({ states, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load loop states";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  loadPatterns: async () => {
    try {
      const patterns = await loopService.getPatterns();
      set({ patterns });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load patterns";
      toast.error(message);
    }
  },

  createState: async (novelId: string, req: CreateLoopStateRequest) => {
    const tempId = `temp-${Date.now()}`;
    const optimistic: LoopState = {
      id: tempId,
      novel_id: novelId,
      pattern_id: req.pattern_id,
      status: "idle",
      readiness_level: req.readiness_level ?? "L0",
      state_payload: {},
      config: (req.config ?? { cadence: "1d", denylist: [], human_gates: [], max_retries: 3 }) as LoopConfig,
      token_usage_today: 0,
      token_cap_daily: req.token_cap_daily ?? 50000,
      last_run_at: null,
      last_run_result: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    set((state) => ({ states: [...state.states, optimistic] }));

    try {
      const state = await loopService.createLoopState(novelId, req);
      set((s) => ({
        states: s.states.map((ls) => (ls.id === tempId ? state : ls)),
      }));
      return state;
    } catch (err) {
      set((s) => ({
        states: s.states.filter((ls) => ls.id !== tempId),
      }));
      const message = err instanceof Error ? err.message : "Failed to create loop state";
      toast.error(message);
      throw err;
    }
  },

  deleteState: async (stateId: string) => {
    const prev = _get().states;
    set((s) => ({
      states: s.states.filter((ls) => ls.id !== stateId),
    }));

    try {
      await loopService.deleteLoopState(stateId);
    } catch (err) {
      set({ states: prev });
      const message = err instanceof Error ? err.message : "Failed to delete loop";
      toast.error(message);
    }
  },

  runCycle: async (stateId: string) => {
    set((s) => ({
      states: s.states.map((ls) =>
        ls.id === stateId ? { ...ls, status: "running" as const } : ls
      ),
    }));

    try {
      const log = await loopService.runLoopCycle(stateId);
      set((s) => ({
        runLogs: [log, ...s.runLogs],
        states: s.states.map((ls) =>
          ls.id === stateId
            ? { ...ls, status: "idle" as const, last_run_at: log.created_at, last_run_result: { findings: log.findings, actions: log.actions_taken, escalations: log.escalations } }
            : ls
        ),
      }));
      return log;
    } catch (err) {
      set((s) => ({
        states: s.states.map((ls) =>
          ls.id === stateId ? { ...ls, status: "error" as const } : ls
        ),
      }));
      const message = err instanceof Error ? err.message : "Failed to run loop cycle";
      toast.error(message);
      throw err;
    }
  },

  pauseLoop: async (stateId: string) => {
    try {
      await loopService.pauseLoop(stateId);
      set((s) => ({
        states: s.states.map((ls) =>
          ls.id === stateId ? { ...ls, status: "paused" as const } : ls
        ),
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to pause loop";
      toast.error(message);
    }
  },

  resumeLoop: async (stateId: string) => {
    try {
      await loopService.resumeLoop(stateId);
      set((s) => ({
        states: s.states.map((ls) =>
          ls.id === stateId ? { ...ls, status: "idle" as const } : ls
        ),
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to resume loop";
      toast.error(message);
    }
  },

  loadRunLogs: async (stateId: string) => {
    try {
      const runLogs = await loopService.getRunLogs(stateId);
      set({ runLogs });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load run logs";
      toast.error(message);
    }
  },
}));
