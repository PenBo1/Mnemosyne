import { create } from "zustand";
import type { LoopState, LoopEngineState, CreateLoopStateRequest, LoopConfig } from "@/features/loop/types";
import * as loopService from "@/features/loop/services";
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
      novelId,
      patternId: req.patternId,
      status: "idle",
      readinessLevel: req.readinessLevel ?? "L0",
      statePayload: {},
      config: (req.config ?? { cadence: "1d", denylist: [], humanGates: [], maxRetries: 3 }) as LoopConfig,
      tokenUsageToday: 0,
      tokenCapDaily: req.tokenCapDaily ?? 50000,
      lastRunAt: null,
      lastRunResult: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
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
        runLogs: [log, ...s.runLogs].slice(0, 200),
        states: s.states.map((ls) =>
          ls.id === stateId
            ? { ...ls, status: "idle" as const, lastRunAt: log.createdAt, lastRunResult: { findings: log.findings, actions: log.actionsTaken, escalations: log.escalations } }
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
