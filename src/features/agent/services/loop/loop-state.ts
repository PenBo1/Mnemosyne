// Loop-Engineering Zustand store —— 管理循环状态和模式。
//
// 职责:
// - 缓存 loop states 和 patterns（避免重复 IPC 调用）
// - 跟踪活跃 loop runner 实例
// - 管理 schedule wakeup 和 monitor 的前端状态
//
// 架构约束:
// - 不包含业务逻辑，只做状态管理
// - IPC 调用委托给 loop-runner.ts

import { create } from "zustand";
import type {
  LoopState,
  LoopPattern,
  LoopRunLog,
  LoopStatus,
  ReadinessLevel,
} from "./loop-patterns";

interface ActiveLoop {
  stateId: string;
  patternId: string;
  startedAt: number;
  nextTickAt?: number;
  iterationCount: number;
}

interface LoopStateStore {
  hydrated: boolean;
  states: Map<string, LoopState>;
  patterns: Map<string, LoopPattern>;
  activeLoops: Map<string, ActiveLoop>;
  runLogs: Map<string, LoopRunLog[]>;

  hydratedAt?: number;

  hydrate: () => Promise<void>;
  loadStatesForNovel: (novelId: string) => Promise<void>;
  loadPatterns: () => Promise<void>;
  loadRunLogs: (stateId: string, limit?: number) => Promise<void>;

  getState: (stateId: string) => LoopState | undefined;
  getStatesForNovel: (novelId: string) => LoopState[];
  getPattern: (patternId: string) => LoopPattern | undefined;
  getPatterns: () => LoopPattern[];
  getActiveLoop: (stateId: string) => ActiveLoop | undefined;
  getRunLogs: (stateId: string) => LoopRunLog[];

  setActiveLoop: (stateId: string, active: ActiveLoop | undefined) => void;
  updateActiveLoopNextTick: (stateId: string, nextTickAt: number) => void;
  incrementActiveLoopIteration: (stateId: string) => void;

  updateStateStatus: (stateId: string, status: LoopStatus) => void;
  updateStateReadiness: (stateId: string, readiness: ReadinessLevel) => void;
  updateStateTokenUsage: (stateId: string, usage: number) => void;

  clear: () => void;
}

export const useLoopStateStore = create<LoopStateStore>((set, get) => ({
  hydrated: false,
  states: new Map(),
  patterns: new Map(),
  activeLoops: new Map(),
  runLogs: new Map(),

  hydrate: async () => {
    if (get().hydrated) return;
    await get().loadPatterns();
    set({ hydrated: true, hydratedAt: Date.now() });
  },

  loadStatesForNovel: async (novelId: string) => {
    const { ipc } = await import("@/services/ipc");
    const states = await ipc<LoopState[]>("loop_get_states", { novelId });
    const statesMap = new Map(get().states);
    for (const state of states) {
      statesMap.set(state.id, state);
    }
    set({ states: statesMap });
  },

  loadPatterns: async () => {
    const { ipc } = await import("@/services/ipc");
    const patterns = await ipc<LoopPattern[]>("loop_get_patterns");
    const patternsMap = new Map<string, LoopPattern>();
    for (const pattern of patterns) {
      patternsMap.set(pattern.id, pattern);
    }
    set({ patterns: patternsMap });
  },

  loadRunLogs: async (stateId: string, limit = 100) => {
    const { ipc } = await import("@/services/ipc");
    const logs = await ipc<LoopRunLog[]>("loop_get_run_logs", {
      stateId,
      limit,
    });
    const runLogsMap = new Map(get().runLogs);
    runLogsMap.set(stateId, logs);
    set({ runLogs: runLogsMap });
  },

  getState: (stateId: string) => get().states.get(stateId),

  getStatesForNovel: (novelId: string) =>
    Array.from(get().states.values()).filter((s) => s.novelId === novelId),

  getPattern: (patternId: string) => get().patterns.get(patternId),

  getPatterns: () => Array.from(get().patterns.values()),

  getActiveLoop: (stateId: string) => get().activeLoops.get(stateId),

  getRunLogs: (stateId: string) => get().runLogs.get(stateId) ?? [],

  setActiveLoop: (stateId: string, active: ActiveLoop | undefined) => {
    const activeLoops = new Map(get().activeLoops);
    if (active) {
      activeLoops.set(stateId, active);
    } else {
      activeLoops.delete(stateId);
    }
    set({ activeLoops });
  },

  updateActiveLoopNextTick: (stateId: string, nextTickAt: number) => {
    const activeLoops = new Map(get().activeLoops);
    const existing = activeLoops.get(stateId);
    if (existing) {
      activeLoops.set(stateId, { ...existing, nextTickAt });
      set({ activeLoops });
    }
  },

  incrementActiveLoopIteration: (stateId: string) => {
    const activeLoops = new Map(get().activeLoops);
    const existing = activeLoops.get(stateId);
    if (existing) {
      activeLoops.set(stateId, {
        ...existing,
        iterationCount: existing.iterationCount + 1,
      });
      set({ activeLoops });
    }
  },

  updateStateStatus: (stateId: string, status: LoopStatus) => {
    const states = new Map(get().states);
    const existing = states.get(stateId);
    if (existing) {
      states.set(stateId, { ...existing, status, updatedAt: new Date().toISOString() });
      set({ states });
    }
  },

  updateStateReadiness: (stateId: string, readiness: ReadinessLevel) => {
    const states = new Map(get().states);
    const existing = states.get(stateId);
    if (existing) {
      states.set(stateId, {
        ...existing,
        readinessLevel: readiness,
        updatedAt: new Date().toISOString(),
      });
      set({ states });
    }
  },

  updateStateTokenUsage: (stateId: string, usage: number) => {
    const states = new Map(get().states);
    const existing = states.get(stateId);
    if (existing) {
      states.set(stateId, {
        ...existing,
        tokenUsageToday: usage,
        updatedAt: new Date().toISOString(),
      });
      set({ states });
    }
  },

  clear: () => {
    set({
      hydrated: false,
      states: new Map(),
      patterns: new Map(),
      activeLoops: new Map(),
      runLogs: new Map(),
    });
  },
}));