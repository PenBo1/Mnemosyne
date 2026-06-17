import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { AiModelConfig } from "@/lib/settings";
import * as settingsStore from "@/lib/settings";
import * as providerService from "@/services/providers";
import * as agentService from "@/services/agent";

export function useModelSettings() {
  const { t } = useI18n();
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
      const message = err instanceof Error ? err.message : t.common.failedToLoadModels;
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, [t.common.failedToLoadModels]);

  useEffect(() => { load(); }, [load]);

  const addModel = useCallback(async (config: Omit<AiModelConfig, "id">) => {
    try {
      await settingsStore.addModel(config);
      await providerService.refreshProviders();
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToAddModel);
    }
  }, [load, t.common.createdSuccessfully, t.common.failedToAddModel]);

  const removeModel = useCallback(async (id: string) => {
    try {
      await settingsStore.removeModel(id);
      await providerService.refreshProviders();
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToDeleteModel);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDeleteModel]);

  const updateModel = useCallback(async (id: string, updates: Partial<Omit<AiModelConfig, "id">>) => {
    try {
      await settingsStore.updateModel(id, updates);
      await providerService.refreshProviders();
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToUpdateModel);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdateModel]);

  const setActiveModel = useCallback(async (id: string) => {
    try {
      await settingsStore.setActiveModel(id);
      await providerService.refreshProviders();
      await agentService.restartAgent();
      setActiveModelId(id);
      toast.success(t.common.updatedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToSetActiveModel);
    }
  }, [t.common.updatedSuccessfully, t.common.failedToSetActiveModel]);

  const testConnection = useCallback(async (params: {
    provider: string;
    apiKey: string;
    baseUrl: string;
    model: string;
  }) => {
    try {
      const result = await providerService.testConnection(params);
      return result;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.error);
      throw err;
    }
  }, [t.common.error]);

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
