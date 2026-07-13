// Custom Agents zustand store。
//
// 职责：
// - 管理 builtin + custom agents 列表
// - 跟踪当前激活 agent id
// - 持久化 custom agents + activeId 到 LazyStore
// - 多窗口同步（P2+ 加 Tauri emit/listen）

import { create } from "zustand";
import {
  BUILTIN_AGENTS,
  loadAgents,
  saveActiveAgentId,
  saveCustomAgents,
  type Agent,
} from "@/features/agent/services/agents";

type AgentsState = {
  hydrated: boolean;
  customAgents: Agent[];
  activeId: string;
  /** builtin + custom 全集。 */
  all: () => Agent[];
  /** 激活的 agent。 */
  active: () => Agent;
  hydrate: () => Promise<void>;
  setActiveId: (id: string) => void;
  upsert: (agent: Agent) => void;
  remove: (id: string) => void;
};

export const useAgentsStore = create<AgentsState>((set, get) => ({
  hydrated: false,
  customAgents: [],
  activeId: BUILTIN_AGENTS[0].id,
  all: () => [...BUILTIN_AGENTS, ...get().customAgents],
  active: () => {
    const all = get().all();
    return all.find((a) => a.id === get().activeId) ?? BUILTIN_AGENTS[0];
  },
  hydrate: async () => {
    if (get().hydrated) return;
    const { custom, activeId } = await loadAgents();
    set({ customAgents: custom, activeId, hydrated: true });
  },
  setActiveId: (id) => {
    set({ activeId: id });
    void saveActiveAgentId(id);
  },
  upsert: (agent) => {
    if (agent.builtIn) return; // builtin 不可改
    const list = get().customAgents;
    const idx = list.findIndex((a) => a.id === agent.id);
    const next = idx === -1
      ? [...list, agent]
      : list.map((a) => (a.id === agent.id ? agent : a));
    set({ customAgents: next });
    void saveCustomAgents(next);
  },
  remove: (id) => {
    const list = get().customAgents.filter((a) => a.id !== id);
    set({ customAgents: list });
    let active = get().activeId;
    if (active === id) {
      active = BUILTIN_AGENTS[0].id;
      set({ activeId: active });
      void saveActiveAgentId(active);
    }
    void saveCustomAgents(list);
  },
}));
