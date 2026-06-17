import { useState, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import * as novelService from "@/services/novel";
import type { BookConfig, WriteResult } from "@/types";

export function useNovelPipeline(workspaceId?: string) {
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentBook, setCurrentBook] = useState<BookConfig | null>(null);
  const [lastResult, setLastResult] = useState<WriteResult | null>(null);

  const createBook = useCallback(async (title: string, genre: string, brief?: string) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const config = await novelService.createNovelPipeline(workspaceId, title, genre, brief);
      setCurrentBook(config);
      toast.success(t.common.createdSuccessfully);
      return config;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.failedToCreate;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.createdSuccessfully, t.common.failedToCreate]);

  const writeNext = useCallback(async (bookId: string, targetWords?: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const result = await novelService.writeNextChapter(workspaceId, bookId, targetWords);
      setLastResult(result);
      toast.success(t.common.updatedSuccessfully);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.failedToUpdate;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const plan = useCallback(async (bookId: string, context?: string) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const intent = await novelService.planChapter(workspaceId, bookId, context);
      return intent;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.error;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.error]);

  const audit = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const result = await novelService.auditChapter(workspaceId, bookId, chapterNumber);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.error;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.error]);

  const revise = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const content = await novelService.reviseChapter(workspaceId, bookId, chapterNumber);
      return content;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.error;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.error]);

  const observe = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const observation = await novelService.observeChapter(workspaceId, bookId, chapterNumber);
      return observation;
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.error;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.error]);

  const reflect = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      await novelService.reflectChapter(workspaceId, bookId, chapterNumber);
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.common.error;
      setError(msg);
      toast.error(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.error]);

  return {
    loading,
    error,
    currentBook,
    lastResult,
    createBook,
    writeNext,
    plan,
    audit,
    revise,
    observe,
    reflect,
  };
}
