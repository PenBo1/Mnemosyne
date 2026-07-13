import { ipc } from "@/services/ipc";
import type {
  PipelineBook, PipelineChapter, CreateBookRequest,
  PlanChapterResult, ComposeChapterResult, DraftResult, AuditResult,
  ReviseResult, ChapterPipelineResult, SchedulerStatus,
} from "@/features/pipeline/types";

// ── 书籍管理 ──
export async function listBooks(): Promise<PipelineBook[]> {
  return ipc<PipelineBook[]>("pipeline_list_books");
}

export async function getBook(bookId: string): Promise<PipelineBook> {
  return ipc<PipelineBook>("pipeline_get_book", { bookId });
}

// ── 章节管理 ──
export async function listChapters(bookId: string): Promise<PipelineChapter[]> {
  return ipc<PipelineChapter[]>("pipeline_list_chapters", { bookId });
}

export async function getChapter(bookId: string, chapterNumber: number): Promise<PipelineChapter> {
  return ipc<PipelineChapter>("pipeline_get_chapter", { bookId, chapterNumber });
}

// ── 真相文件 ──
export async function readTruthFile(bookId: string, kind: string): Promise<string> {
  return ipc<string>("pipeline_read_truth_file", { bookId, kind });
}

// ── 核心编排（8-agent cycle）──
export async function initBook(req: CreateBookRequest): Promise<void> {
  await ipc<unknown>("pipeline_init_book", {
    title: req.title,
    platform: req.platform,
    genre: req.genre,
    targetChapters: req.target_chapters,
    chapterWordCount: req.chapter_word_count,
    language: req.language,
    authorIntent: req.author_intent,
    externalContext: req.external_context,
  });
}

export async function reviseFoundation(bookId: string, feedback: string): Promise<void> {
  await ipc<unknown>("pipeline_revise_foundation", { bookId, feedback });
}

export async function planChapter(bookId: string): Promise<PlanChapterResult> {
  return ipc<PlanChapterResult>("pipeline_plan_chapter", { bookId });
}

export async function composeChapter(bookId: string): Promise<ComposeChapterResult> {
  return ipc<ComposeChapterResult>("pipeline_compose_chapter", { bookId });
}

export async function writeDraft(bookId: string, wordCountOverride?: number): Promise<DraftResult> {
  return ipc<DraftResult>("pipeline_write_draft", { bookId, wordCountOverride });
}

export async function auditDraft(bookId: string, chapterNumber: number): Promise<AuditResult> {
  return ipc<AuditResult>("pipeline_audit_draft", { bookId, chapterNumber });
}

export async function reviseDraft(
  bookId: string,
  chapterNumber: number,
  mode: "auto" | "manual",
): Promise<ReviseResult> {
  return ipc<ReviseResult>("pipeline_revise_draft", { bookId, chapterNumber, mode });
}

export async function writeNextChapter(
  bookId: string,
  wordCountOverride?: number,
): Promise<ChapterPipelineResult> {
  return ipc<ChapterPipelineResult>("pipeline_write_next_chapter", {
    bookId,
    wordCountOverride,
  });
}

export async function consolidate(bookId: string): Promise<unknown> {
  return ipc<unknown>("pipeline_consolidate", { bookId });
}

// ── 调度器 ──
export async function schedulerStart(): Promise<void> {
  await ipc<unknown>("pipeline_scheduler_start");
}

export async function schedulerStop(): Promise<void> {
  await ipc<unknown>("pipeline_scheduler_stop");
}

export async function schedulerStatus(): Promise<SchedulerStatus> {
  return ipc<SchedulerStatus>("pipeline_scheduler_status");
}

export async function schedulerTriggerWrite(): Promise<void> {
  await ipc<unknown>("pipeline_scheduler_trigger_write");
}

export async function schedulerTriggerRadar(): Promise<void> {
  await ipc<unknown>("pipeline_scheduler_trigger_radar");
}

export async function schedulerResumeBook(bookId: string): Promise<void> {
  await ipc<unknown>("pipeline_scheduler_resume_book", { bookId });
}

export async function schedulerIsBookPaused(bookId: string): Promise<boolean> {
  return ipc<boolean>("pipeline_scheduler_is_book_paused", { bookId });
}
