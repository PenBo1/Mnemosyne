import { useState, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import * as wikiService from "@/features/wiki/services";
import type {
  WikiEntry,
  WikiGraphView,
  CreateWikiEntryRequest,
  UpdateWikiEntryRequest,
  WikiCategory,
} from "@/shared/types";

export function useWiki(novelId?: string) {
  const { t } = useI18n();
  const [entries, setEntries] = useState<WikiEntry[]>([]);
  const [graph, setGraph] = useState<WikiGraphView | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadEntries = useCallback(
    async (category?: WikiCategory) => {
      if (!novelId) return;
      setLoading(true);
      setError(null);
      try {
        const list = await wikiService.listWikiEntries(novelId, category);
        setEntries(list);
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
      } finally {
        setLoading(false);
      }
    },
    [novelId, t.common.error]
  );

  const loadGraph = useCallback(
    async (category?: WikiCategory, minImportance?: number) => {
      if (!novelId) return;
      setLoading(true);
      setError(null);
      try {
        const view = await wikiService.getWikiGraph(novelId, category, minImportance);
        setGraph(view);
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
      } finally {
        setLoading(false);
      }
    },
    [novelId, t.common.error]
  );

  const createEntry = useCallback(
    async (request: CreateWikiEntryRequest) => {
      if (!novelId) throw new Error("No novel selected");
      setLoading(true);
      setError(null);
      try {
        const entry = await wikiService.createWikiEntry(novelId, request);
        setEntries((prev) => [...prev, entry]);
        toast.success(t.common.createdSuccessfully);
        return entry;
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.failedToCreate;
        setError(msg);
        toast.error(msg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [novelId, t.common.createdSuccessfully, t.common.failedToCreate]
  );

  const updateEntry = useCallback(
    async (entryId: string, request: UpdateWikiEntryRequest) => {
      setLoading(true);
      setError(null);
      try {
        const entry = await wikiService.updateWikiEntry(entryId, request);
        setEntries((prev) => prev.map((e) => (e.id === entryId ? entry : e)));
        toast.success(t.common.updatedSuccessfully);
        return entry;
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.failedToUpdate;
        setError(msg);
        toast.error(msg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [t.common.updatedSuccessfully, t.common.failedToUpdate]
  );

  const deleteEntry = useCallback(
    async (entryId: string) => {
      setLoading(true);
      setError(null);
      try {
        await wikiService.deleteWikiEntry(entryId);
        setEntries((prev) => prev.filter((e) => e.id !== entryId));
        toast.success(t.common.deletedSuccessfully);
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.failedToDelete;
        setError(msg);
        toast.error(msg);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [t.common.deletedSuccessfully, t.common.failedToDelete]
  );

  const createLink = useCallback(
    async (
      sourceId: string,
      targetId: string,
      linkType: string,
      description?: string
    ) => {
      if (!novelId) throw new Error("No novel selected");
      try {
        const link = await wikiService.createWikiLink(
          novelId,
          sourceId,
          targetId,
          linkType,
          description
        );
        // 重新加载图谱以反映新链接
        await loadGraph();
        return link;
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.error;
        toast.error(msg);
        throw err;
      }
    },
    [novelId, loadGraph, t.common.error]
  );

  const deleteLink = useCallback(
    async (linkId: string) => {
      try {
        await wikiService.deleteWikiLink(linkId);
        // 重新加载图谱以反映已删除的链接
        await loadGraph();
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.error;
        toast.error(msg);
        throw err;
      }
    },
    [loadGraph, t.common.error]
  );

  const search = useCallback(
    async (query: string, limit?: number) => {
      if (!novelId) return [];
      setLoading(true);
      setError(null);
      try {
        const results = await wikiService.searchWikiEntries(novelId, query, limit);
        return results;
      } catch (err) {
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
        return [];
      } finally {
        setLoading(false);
      }
    },
    [novelId, t.common.error]
  );

  return {
    entries,
    graph,
    loading,
    error,
    loadEntries,
    loadGraph,
    createEntry,
    updateEntry,
    deleteEntry,
    createLink,
    deleteLink,
    search,
  };
}