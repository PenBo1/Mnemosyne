// Memory hook —— 连接 services/memory 与 React 状态。
//
// 使用方式：
// ```tsx
// const activeBookId = useWorkspaceStore((s) => s.activeWorkspaceId);
// const { memories, loading, create, update, remove, refresh } = useMemory(activeBookId);
// ```

import { useState, useMemo, useCallback, useEffect } from "react";
import { useI18n } from "@/shared/i18n";
import { useAsyncAction } from "@/core/memory/useAsyncAction";
import * as memoryService from "@/features/memory/services";
import { MEMORY_TYPES } from "@/shared/types/memory";
import type { MemoryEntry, MemoryStats, MemoryType } from "@/shared/types/memory";

export function useMemory(bookId: string | null | undefined) {
  const { t } = useI18n();
  const [memories, setMemories] = useState<MemoryEntry[]>([]);
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [filterEntryType, setFilterEntryType] = useState<MemoryType | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const { loading, error, run } = useAsyncAction();

  const refresh = useCallback(async () => {
    if (!bookId) return;
    const list = await run(() => memoryService.listMemories(bookId), {
      errorToast: t.common.error,
    });
    if (list) setMemories(list);
  }, [bookId, run, t.common.error]);

  const refreshStats = useCallback(async () => {
    if (!bookId) return;
    const s = await run(() => memoryService.getMemoryStats(bookId), {
      errorToast: t.common.error,
    });
    if (s) setStats(s);
  }, [bookId, run, t.common.error]);

  // bookId 变化时自动加载
  useEffect(() => {
    if (bookId) {
      refresh();
      refreshStats();
    } else {
      setMemories([]);
      setStats(null);
    }
  }, [bookId, refresh, refreshStats]);

  // 远端搜索（BM25）—— 空查询时退回全量加载
  const search = useCallback(
    async (query: string, topK = 10) => {
      if (!bookId) return;
      if (!query.trim()) {
        await refresh();
        return;
      }
      const results = await run(
        () => memoryService.searchMemories(bookId, query, topK),
        { errorToast: t.common.error },
      );
      if (results) setMemories(results);
    },
    [bookId, run, t.common.error, refresh],
  );

  const create = useCallback(
    async (params: {
      content: string;
      entryType: MemoryType;
      chapter?: number | null;
      tags?: string[];
    }) => {
      if (!bookId) return null;
      const entry = await run(
        () => memoryService.createMemory({ bookId, ...params }),
        {
          successToast: t.common.createdSuccessfully,
          errorToast: t.common.failedToCreate,
        },
      );
      if (entry) {
        setMemories((prev) => [entry, ...prev]);
        // 统计也变化了，异步刷新
        refreshStats();
      }
      return entry;
    },
    [bookId, run, t.common.createdSuccessfully, t.common.failedToCreate, refreshStats],
  );

  const update = useCallback(
    async (entryId: string, content: string, tags: string[]) => {
      if (!bookId) return null;
      const entry = await run(
        () => memoryService.updateMemory(bookId, entryId, content, tags),
        {
          successToast: t.common.updatedSuccessfully,
          errorToast: t.common.failedToUpdate,
        },
      );
      if (entry) {
        setMemories((prev) => prev.map((m) => (m.id === entryId ? entry : m)));
      }
      return entry;
    },
    [bookId, run, t.common.updatedSuccessfully, t.common.failedToUpdate],
  );

  const remove = useCallback(
    async (entryId: string) => {
      if (!bookId) return false;
      const success = await run(
        () => memoryService.deleteMemory(bookId, entryId),
        {
          successToast: t.common.deletedSuccessfully,
          errorToast: t.common.failedToDelete,
        },
      );
      if (success) {
        setMemories((prev) => prev.filter((m) => m.id !== entryId));
        refreshStats();
      }
      return success ?? false;
    },
    [bookId, run, t.common.deletedSuccessfully, t.common.failedToDelete, refreshStats],
  );

  // 本地过滤（基于 filterEntryType + searchQuery）
  const filtered = useMemo(() => {
    const q = searchQuery.toLowerCase();
    return memories.filter((m) => {
      const matchesType = filterEntryType === "all" || m.entry_type === filterEntryType;
      const matchesSearch =
        !q ||
        m.content.toLowerCase().includes(q) ||
        m.tags.some((tag) => tag.toLowerCase().includes(q));
      return matchesType && matchesSearch;
    });
  }, [memories, filterEntryType, searchQuery]);

  return {
    memories: filtered,
    allMemories: memories,
    stats,
    loading,
    error,
    filterEntryType,
    setFilterEntryType,
    searchQuery,
    setSearchQuery,
    refresh,
    refreshStats,
    search,
    create,
    update,
    remove,
    types: MEMORY_TYPES,
  };
}
