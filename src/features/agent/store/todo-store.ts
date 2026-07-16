// Todo store —— 按 session 管理内存中的 todo 列表 + 异步持久化。

import { create } from "zustand";
import {
  deleteTodos as persistDelete,
  loadTodos as persistLoad,
  saveTodos as persistSave,
  type Todo,
} from "../services/todos";

type TodosState = {
  /** sessionId -> todos 映射。 */
  bySession: Record<string, Todo[]>;
  /** 已 hydrate 的 sessionId 集合（避免重复加载）。 */
  hydrated: Set<string>;
  hydrate: (sessionId: string) => Promise<void>;
  setTodos: (sessionId: string, todos: Todo[]) => void;
  clearSession: (sessionId: string) => Promise<void>;
};

export const useTodosStore = create<TodosState>((set, get) => ({
  bySession: {},
  hydrated: new Set(),

  async hydrate(sessionId) {
    if (get().hydrated.has(sessionId)) return;
    const todos = await persistLoad(sessionId);
    set((s) => {
      const nextHydrated = new Set(s.hydrated);
      nextHydrated.add(sessionId);
      let bySession = { ...s.bySession, [sessionId]: todos };
      // LRU: 超过 10 个 session 时移除最早的
      if (nextHydrated.size > 10) {
        const oldest = nextHydrated.values().next().value;
        if (oldest && oldest !== sessionId) {
          nextHydrated.delete(oldest);
          delete bySession[oldest];
        }
      }
      return { bySession, hydrated: nextHydrated };
    });
  },

  setTodos(sessionId, todos) {
    set((s) => ({
      bySession: { ...s.bySession, [sessionId]: todos },
    }));
    void persistSave(sessionId, todos);
  },

  async clearSession(sessionId) {
    set((s) => {
      const next = { ...s.bySession };
      delete next[sessionId];
      const nextHydrated = new Set(s.hydrated);
      nextHydrated.delete(sessionId);
      return { bySession: next, hydrated: nextHydrated };
    });
    await persistDelete(sessionId);
  },
}));

export function getTodos(sessionId: string | null): Todo[] {
  if (!sessionId) return [];
  return useTodosStore.getState().bySession[sessionId] ?? [];
}
