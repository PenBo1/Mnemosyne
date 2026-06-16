import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import type { Prompt } from "@/types";
import * as promptsService from "@/services/prompts";

export function usePrompts(filterCategory?: string) {
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await promptsService.fetchPrompts(filterCategory);
      setPrompts(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load prompts";
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, [filterCategory]);

  useEffect(() => {
    load();
  }, [load]);

  const create = useCallback(async (name: string, content: string, category: string) => {
    setError(null);
    try {
      await promptsService.createPrompt(name, content, category, []);
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to create prompt";
      setError(message);
      toast.error(message);
      throw err;
    }
  }, [load]);

  const update = useCallback(async (id: string, name: string, content: string, category: string) => {
    setError(null);
    try {
      await promptsService.updatePrompt(id, name, content, category);
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to update prompt";
      setError(message);
      toast.error(message);
      throw err;
    }
  }, [load]);

  const remove = useCallback(async (id: string) => {
    setError(null);
    try {
      await promptsService.deletePrompt(id);
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to delete prompt";
      setError(message);
      toast.error(message);
      throw err;
    }
  }, [load]);

  return { prompts, loading, error, create, update, remove, reload: load };
}
