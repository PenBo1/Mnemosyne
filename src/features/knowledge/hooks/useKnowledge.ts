import { useState, useEffect, useCallback, useMemo } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { KnowledgeEntry } from "@/features/knowledge/types";
import {
  loadEntries,
  createEntry,
  updateEntry,
  deleteEntry,
} from "@/features/knowledge/services";

export function useKnowledge() {
  const { t } = useI18n();
  const [entries, setEntries] = useState<KnowledgeEntry[]>([]);
  const [filterCategory, setFilterCategory] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const data = await loadEntries();
      setEntries(data);
    } catch {
      setEntries([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, []);

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
      await createEntry(params);
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [load]);

  const update = useCallback(async (id: string, params: {
    title: string;
    content: string;
    category: string;
    tags: string[];
  }) => {
    try {
      await updateEntry(id, params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load]);

  const remove = useCallback(async (id: string) => {
    try {
      await deleteEntry(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load]);

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
