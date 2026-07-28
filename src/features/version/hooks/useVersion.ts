import { useState, useCallback, useRef } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import * as versionService from "@/features/version/services";
import type { ChapterVersion, LineDiffResult } from "@/features/version/types";

export function useVersion(novelId?: string) {
  const { t } = useI18n();
  const [versions, setVersions] = useState<ChapterVersion[]>([]);
  const [diffResult, setDiffResult] = useState<LineDiffResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // 请求序列号：快速切换章节时防止旧响应覆盖新请求的数据
  const requestCounterRef = useRef(0);

  const loadVersions = useCallback(
    async (chapterNumber: number) => {
      if (!novelId) return;
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const list = await versionService.listChapterVersions(novelId, chapterNumber);
        if (requestId !== requestCounterRef.current) return;
        setVersions(list);
      } catch (err) {
        if (requestId !== requestCounterRef.current) return;
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    [novelId]
  );

  const getVersion = useCallback(
    async (versionId: string) => {
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const version = await versionService.getChapterVersion(versionId);
        return version;
      } catch (err) {
        if (requestId !== requestCounterRef.current) return null;
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
        return null;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    []
  );

  const getLatestVersion = useCallback(
    async (chapterNumber: number) => {
      if (!novelId) return null;
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const version = await versionService.getLatestChapterVersion(novelId, chapterNumber);
        return version;
      } catch (err) {
        if (requestId !== requestCounterRef.current) return null;
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
        return null;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    [novelId]
  );

  const computeDiff = useCallback(
    async (fromVersionId: string, toVersionId: string) => {
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const diff = await versionService.computeVersionDiff(fromVersionId, toVersionId);
        if (requestId !== requestCounterRef.current) return diff;
        setDiffResult(diff);
        return diff;
      } catch (err) {
        if (requestId !== requestCounterRef.current) return null;
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
        return null;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    []
  );

  const computeLatestDiff = useCallback(
    async (chapterNumber: number) => {
      if (!novelId) return null;
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const diff = await versionService.computeLatestDiff(novelId, chapterNumber);
        if (requestId !== requestCounterRef.current) return diff;
        setDiffResult(diff);
        return diff;
      } catch (err) {
        if (requestId !== requestCounterRef.current) return null;
        const msg = err instanceof Error ? err.message : t.common.error;
        setError(msg);
        toast.error(msg);
        return null;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    [novelId]
  );

  const restoreVersion = useCallback(
    async (versionId: string, workspaceId: string, bookId: string) => {
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const newVersion = await versionService.restoreVersion(versionId, workspaceId, bookId);
        if (requestId !== requestCounterRef.current) return newVersion;
        // 恢复成功：将新版本 prepend 到列表顶部
        setVersions((prev) => [newVersion, ...prev]);
        toast.success(t.common.updatedSuccessfully);
        return newVersion;
      } catch (err) {
        if (requestId !== requestCounterRef.current) return null;
        const msg = err instanceof Error ? err.message : t.common.failedToUpdate;
        setError(msg);
        toast.error(msg);
        return null;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    []
  );

  const saveVersion = useCallback(
    async (
      chapterNumber: number,
      content: string,
      revisionMode: string = "manual",
      revisionReason: string = "User save"
    ) => {
      if (!novelId) throw new Error("No novel selected");
      const requestId = ++requestCounterRef.current;
      setLoading(true);
      setError(null);
      try {
        const version = await versionService.saveVersion(
          novelId,
          chapterNumber,
          content,
          revisionMode,
          revisionReason
        );
        if (requestId !== requestCounterRef.current) return version;
        setVersions((prev) => [version, ...prev]);
        toast.success(t.common.createdSuccessfully);
        return version;
      } catch (err) {
        if (requestId !== requestCounterRef.current) throw err;
        const msg = err instanceof Error ? err.message : t.common.failedToCreate;
        setError(msg);
        toast.error(msg);
        throw err;
      } finally {
        if (requestId === requestCounterRef.current) setLoading(false);
      }
    },
    [novelId]
  );

  return {
    versions,
    diffResult,
    loading,
    error,
    loadVersions,
    getVersion,
    getLatestVersion,
    computeDiff,
    computeLatestDiff,
    restoreVersion,
    saveVersion,
  };
}