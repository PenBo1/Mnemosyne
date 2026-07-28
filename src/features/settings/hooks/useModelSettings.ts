import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { AiModelConfig } from "@/services/settings";
import {
  loadSettings,
  addModel as addModelSetting,
  removeModel as removeModelSetting,
  updateModel as updateModelSetting,
  setActiveModel as setActiveModelSetting,
} from "@/services/settings";
import {
  refreshProviders,
  testConnection as testProviderConnection,
} from "@/features/settings/services";

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
      const settings = await loadSettings();
      setModels(settings.ai.models);
      setActiveModelId(settings.ai.active_model_id);
    } catch (err) {
      const message = err instanceof Error ? err.message : t.common.failedToLoadModels;
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const addModel = useCallback(async (config: Omit<AiModelConfig, "id">) => {
    try {
      await addModelSetting(config);
      await refreshProviders();
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToAddModel);
    }
  }, [load]);

  const removeModel = useCallback(async (id: string) => {
    try {
      await removeModelSetting(id);
      await refreshProviders();
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToDeleteModel);
    }
  }, [load]);

  const updateModel = useCallback(async (id: string, updates: Partial<Omit<AiModelConfig, "id">>) => {
    try {
      await updateModelSetting(id, updates);
      await refreshProviders();
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToUpdateModel);
    }
  }, [load]);

  const setActiveModel = useCallback(async (id: string) => {
    try {
      await setActiveModelSetting(id);
      await refreshProviders();
      setActiveModelId(id);
      toast.success(t.common.updatedSuccessfully);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToSetActiveModel);
    }
  }, []);

  const testConnection = useCallback(async (params: {
    provider: string;
    apiKey: string;
    baseUrl: string;
    model: string;
  }) => {
    try {
      const result = await testProviderConnection(params);
      return result;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.error);
      throw err;
    }
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
