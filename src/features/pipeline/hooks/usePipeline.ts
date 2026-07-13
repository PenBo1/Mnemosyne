import { useEffect, useCallback } from "react";
import { usePipelineStore } from "@/features/pipeline/store";

export function usePipeline() {
  const books = usePipelineStore((s) => s.books);
  const chapters = usePipelineStore((s) => s.chapters);
  const schedulerStatus = usePipelineStore((s) => s.schedulerStatus);
  const selectedBookId = usePipelineStore((s) => s.selectedBookId);
  const loading = usePipelineStore((s) => s.loading);
  const running = usePipelineStore((s) => s.running);
  const error = usePipelineStore((s) => s.error);

  const loadBooks = usePipelineStore((s) => s.loadBooks);
  const selectBook = usePipelineStore((s) => s.selectBook);
  const loadChapters = usePipelineStore((s) => s.loadChapters);
  const loadSchedulerStatus = usePipelineStore((s) => s.loadSchedulerStatus);
  const createBook = usePipelineStore((s) => s.createBook);
  const writeNextChapter = usePipelineStore((s) => s.writeNextChapter);
  const startScheduler = usePipelineStore((s) => s.startScheduler);
  const stopScheduler = usePipelineStore((s) => s.stopScheduler);
  const triggerWriteCycle = usePipelineStore((s) => s.triggerWriteCycle);
  const resumeBook = usePipelineStore((s) => s.resumeBook);

  useEffect(() => {
    loadBooks();
    loadSchedulerStatus();
  }, [loadBooks, loadSchedulerStatus]);

  const handleSelectBook = useCallback(
    (bookId: string | null) => {
      selectBook(bookId);
    },
    [selectBook],
  );

  const handleWriteNextChapter = useCallback(
    (bookId: string) => {
      writeNextChapter(bookId);
    },
    [writeNextChapter],
  );

  return {
    books,
    chapters,
    schedulerStatus,
    selectedBookId,
    loading,
    running,
    error,
    loadBooks,
    selectBook: handleSelectBook,
    loadChapters,
    loadSchedulerStatus,
    createBook,
    writeNextChapter: handleWriteNextChapter,
    startScheduler,
    stopScheduler,
    triggerWriteCycle,
    resumeBook,
  };
}
