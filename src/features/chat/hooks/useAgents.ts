import { useEffect } from "react";
import { useAgentConfigStore, AVAILABLE_MODELS } from "@/stores/agent-config";

export function useAgents() {
  const agents = useAgentConfigStore((s) => s.agents);
  const loading = useAgentConfigStore((s) => s.loading);
  const error = useAgentConfigStore((s) => s.error);
  const loadAgents = useAgentConfigStore((s) => s.loadAgents);
  const updateAgent = useAgentConfigStore((s) => s.updateAgent);
  const toggleAgentStatus = useAgentConfigStore((s) => s.toggleAgentStatus);

  useEffect(() => {
    loadAgents();
  }, [loadAgents]);

  return {
    agents,
    models: AVAILABLE_MODELS,
    loading,
    error,
    update: updateAgent,
    toggleStatus: toggleAgentStatus,
  };
}
