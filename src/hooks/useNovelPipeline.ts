import { useState, useCallback } from "react";
import * as novelService from "@/services/novel";
import type { BookConfig, WriteResult } from "@/types";

export function useNovelPipeline(workspaceId?: string) {
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
      return config;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to create book";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const writeNext = useCallback(async (bookId: string, targetWords?: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const result = await novelService.writeNextChapter(workspaceId, bookId, targetWords);
      setLastResult(result);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to write chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const plan = useCallback(async (bookId: string, context?: string) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const intent = await novelService.planChapter(workspaceId, bookId, context);
      return intent;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to plan chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const audit = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const result = await novelService.auditChapter(workspaceId, bookId, chapterNumber);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to audit chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const revise = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const content = await novelService.reviseChapter(workspaceId, bookId, chapterNumber);
      return content;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to revise chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const observe = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      const observation = await novelService.observeChapter(workspaceId, bookId, chapterNumber);
      return observation;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to observe chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  const reflect = useCallback(async (bookId: string, chapterNumber: number) => {
    if (!workspaceId) throw new Error("No workspace selected");
    setLoading(true);
    setError(null);
    try {
      await novelService.reflectChapter(workspaceId, bookId, chapterNumber);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to reflect chapter";
      setError(msg);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

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
