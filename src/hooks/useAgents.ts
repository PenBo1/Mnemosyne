import { useEffect } from "react";
import { useAgentConfigStore, AVAILABLE_MODELS } from "@/stores/agent-config";

export function useAgents() {
  const {
    agents,
    loading,
    error,
    loadAgents,
    updateAgent,
    toggleAgentStatus,
    getIdentity,
    updateIdentity,
  } = useAgentConfigStore();

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
    getIdentity,
    updateIdentity,
  };
}
