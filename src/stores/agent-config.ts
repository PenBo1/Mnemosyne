import { create } from "zustand";
import { toast } from "sonner";
import type { Agent, AgentIdentity } from "@/types";
import { AVAILABLE_MODELS } from "@/constants";
import * as agentService from "@/services/agent";

interface AgentConfigState {
  agents: Agent[];
  loading: boolean;
  error: string | null;
  loadAgents: () => Promise<void>;
  updateAgent: (id: string, updates: Partial<Agent>) => Promise<void>;
  toggleAgentStatus: (id: string) => Promise<void>;
  getIdentity: (role: string) => Promise<AgentIdentity | null>;
  updateIdentity: (role: string, updates: { soul?: string; context?: string; memory?: string }) => Promise<AgentIdentity | null>;
}

export const useAgentConfigStore = create<AgentConfigState>((set) => ({
  agents: [],
  loading: false,
  error: null,

  loadAgents: async () => {
    set({ loading: true, error: null });
    try {
      const agents = await agentService.fetchAgents();
      set({ agents, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load agents";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  updateAgent: async (id, updates) => {
    try {
      const updated = await agentService.updateAgent(id, updates);
      set((state) => ({
        agents: state.agents.map((a) => (a.id === id ? updated : a)),
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to update agent";
      set({ error: message });
      toast.error(message);
    }
  },

  toggleAgentStatus: async (id) => {
    try {
      const updated = await agentService.toggleAgentStatus(id);
      set((state) => ({
        agents: state.agents.map((a) => (a.id === id ? updated : a)),
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to toggle agent status";
      set({ error: message });
      toast.error(message);
    }
  },

  getIdentity: async (role) => {
    try {
      return await agentService.getAgentIdentity(role);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load identity";
      toast.error(message);
      return null;
    }
  },

  updateIdentity: async (role, updates) => {
    try {
      const updated = await agentService.updateAgentIdentity(role, updates);
      toast.success("Identity updated");
      return updated;
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to update identity";
      toast.error(message);
      return null;
    }
  },
}));

export { AVAILABLE_MODELS };
