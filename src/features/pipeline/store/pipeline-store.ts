import { create } from "zustand";
import { toast } from "sonner";
import type {
  PipelineBook, PipelineChapter, SchedulerStatus, CreateBookRequest,
} from "@/features/pipeline/types";
import * as pipelineService from "@/features/pipeline/services";

interface PipelineState {
  // 状态
  books: PipelineBook[];
  chapters: PipelineChapter[];  // 当前选中书籍的章节
  schedulerStatus: SchedulerStatus | null;
  selectedBookId: string | null;
  loading: boolean;
  running: boolean;
  error: string | null;

  // Actions
  loadBooks: () => Promise<void>;
  selectBook: (bookId: string | null) => Promise<void>;
  loadChapters: (bookId: string) => Promise<void>;
  loadSchedulerStatus: () => Promise<void>;
  createBook: (req: CreateBookRequest) => Promise<void>;
  writeNextChapter: (bookId: string) => Promise<void>;
  startScheduler: () => Promise<void>;
  stopScheduler: () => Promise<void>;
  triggerWriteCycle: () => Promise<void>;
  resumeBook: (bookId: string) => Promise<void>;
}

export const usePipelineStore = create<PipelineState>((set, get) => ({
  books: [],
  chapters: [],
  schedulerStatus: null,
  selectedBookId: null,
  loading: false,
  running: false,
  error: null,

  loadBooks: async () => {
    set({ loading: true, error: null });
    try {
      const books = await pipelineService.listBooks();
      set({ books, loading: false });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load books";
      set({ error: msg, loading: false });
      toast.error(msg);
    }
  },

  selectBook: async (bookId) => {
    set({ selectedBookId: bookId });
    if (bookId) {
      await get().loadChapters(bookId);
    } else {
      set({ chapters: [] });
    }
  },

  loadChapters: async (bookId) => {
    set({ loading: true, error: null });
    try {
      const chapters = await pipelineService.listChapters(bookId);
      set({ chapters, loading: false });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load chapters";
      set({ error: msg, loading: false });
      toast.error(msg);
    }
  },

  loadSchedulerStatus: async () => {
    try {
      const status = await pipelineService.schedulerStatus();
      set({ schedulerStatus: status });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load scheduler status";
      toast.error(msg);
    }
  },

  createBook: async (req) => {
    set({ running: true, error: null });
    try {
      await pipelineService.initBook(req);
      await get().loadBooks();
      set({ running: false });
      toast.success("Book created");
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to create book";
      set({ error: msg, running: false });
      toast.error(msg);
    }
  },

  writeNextChapter: async (bookId) => {
    set({ running: true, error: null });
    try {
      const result = await pipelineService.writeNextChapter(bookId);
      await get().loadChapters(bookId);
      set({ running: false });
      toast.success(`Chapter ${result.chapter_number} completed`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to write chapter";
      set({ error: msg, running: false });
      toast.error(msg);
    }
  },

  startScheduler: async () => {
    try {
      await pipelineService.schedulerStart();
      await get().loadSchedulerStatus();
      toast.success("Scheduler started");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to start scheduler");
    }
  },

  stopScheduler: async () => {
    try {
      await pipelineService.schedulerStop();
      await get().loadSchedulerStatus();
      toast.success("Scheduler stopped");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to stop scheduler");
    }
  },

  triggerWriteCycle: async () => {
    try {
      await pipelineService.schedulerTriggerWrite();
      toast.success("Write cycle triggered");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to trigger write cycle");
    }
  },

  resumeBook: async (bookId) => {
    try {
      await pipelineService.schedulerResumeBook(bookId);
      await get().loadSchedulerStatus();
      toast.success("Book resumed");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to resume book");
    }
  },
}));
