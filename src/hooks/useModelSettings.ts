import { useState, useEffect, useCallback } from "react";
import type { AiModelConfig } from "@/lib/settings";
import * as settingsStore from "@/lib/settings";
import * as providerService from "@/services/providers";
import * as agentService from "@/services/agent";

export function useModelSettings() {
  const [models, setModels] = useState<AiModelConfig[]>([]);
  const [activeModelId, setActiveModelId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const settings = await settingsStore.loadSettings();
      setModels(settings.ai.models);
      setActiveModelId(settings.ai.active_model_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load models");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const addModel = useCallback(async (config: Omit<AiModelConfig, "id">) => {
    await settingsStore.addModel(config);
    await providerService.refreshProviders();
    await load();
  }, [load]);

  const removeModel = useCallback(async (id: string) => {
    await settingsStore.removeModel(id);
    await providerService.refreshProviders();
    await load();
  }, [load]);

  const updateModel = useCallback(async (id: string, updates: Partial<Omit<AiModelConfig, "id">>) => {
    await settingsStore.updateModel(id, updates);
    await providerService.refreshProviders();
    await load();
  }, [load]);

  const setActiveModel = useCallback(async (id: string) => {
    await settingsStore.setActiveModel(id);
    await providerService.refreshProviders();
    await agentService.restartAgent();
    setActiveModelId(id);
  }, []);

  const testConnection = useCallback(async (params: {
    provider: string;
    apiKey: string;
    baseUrl: string;
    model: string;
  }) => {
    await providerService.testConnection(params);
  }, []);

  return {
    models,
    activeModelId,
    loading,
    error,
    addModel,
    removeModel,
    updateModel,
    setActiveModel,
    testConnection,
    reload: load,
  };
}
