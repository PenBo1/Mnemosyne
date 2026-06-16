import { useState, useEffect, useCallback, useMemo } from "react";
import type { Novel } from "@/types";
import * as novelsService from "@/services/novel";

export function useNovels(workspaceId?: string) {
  const [novels, setNovels] = useState<Novel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await novelsService.fetchNovels();
      setNovels(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load novels";
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const filteredNovels = useMemo(() => {
    if (!workspaceId) return [];
    return novels.filter((n) => n.workspace_id === workspaceId);
  }, [novels, workspaceId]);

  const create = useCallback(async (title: string, genre: string) => {
    if (!workspaceId) {
      throw new Error("No workspace selected");
    }
    setError(null);
    try {
      await novelsService.createNovelList(workspaceId, title, genre);
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to create novel";
      setError(message);
      throw err;
    }
  }, [workspaceId, load]);

  const remove = useCallback(async (id: string) => {
    setError(null);
    try {
      await novelsService.deleteNovel(id);
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to delete novel";
      setError(message);
      throw err;
    }
  }, [load]);

  return { novels: filteredNovels, loading, error, create, remove, reload: load };
}
