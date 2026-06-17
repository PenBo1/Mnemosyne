import { useState, useEffect, useCallback, useMemo } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { KnowledgeEntry } from "@/types";
import * as knowledgeService from "@/services/knowledge";

export function useKnowledge() {
  const { t } = useI18n();
  const [entries, setEntries] = useState<KnowledgeEntry[]>([]);
  const [filterCategory, setFilterCategory] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const data = await knowledgeService.loadEntries();
      setEntries(data);
    } catch {
      setEntries([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [t.common.failedToLoad]);

  useEffect(() => { load(); }, [load]);

  const filteredEntries = useMemo(() => {
    return entries.filter((entry) => {
      const matchesCategory = filterCategory === "all" || entry.category === filterCategory;
      const matchesSearch =
        searchQuery === "" ||
        entry.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        entry.content.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  }, [entries, filterCategory, searchQuery]);

  const create = useCallback(async (params: {
    title: string;
    content: string;
    category: string;
    tags: string[];
  }) => {
    try {
      await knowledgeService.createEntry(params);
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [load, t.common.createdSuccessfully, t.common.failedToCreate]);

  const update = useCallback(async (id: string, params: {
    title: string;
    content: string;
    category: string;
    tags: string[];
  }) => {
    try {
      await knowledgeService.updateEntry(id, params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await knowledgeService.deleteEntry(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return {
    entries: filteredEntries,
    allEntries: entries,
    loading,
    filterCategory,
    setFilterCategory,
    searchQuery,
    setSearchQuery,
    create,
    update,
    remove,
    reload: load,
  };
}
