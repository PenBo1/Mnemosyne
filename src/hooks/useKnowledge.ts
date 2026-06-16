import { useState, useEffect, useCallback, useMemo } from "react";
import type { KnowledgeEntry } from "@/types";
import * as knowledgeService from "@/services/knowledge";

export function useKnowledge() {
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
    await knowledgeService.createEntry(params);
    await load();
  }, [load]);

  const update = useCallback(async (id: string, params: {
    title: string;
    content: string;
    category: string;
    tags: string[];
  }) => {
    await knowledgeService.updateEntry(id, params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await knowledgeService.deleteEntry(id);
    await load();
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
