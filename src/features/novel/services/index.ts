import { ipc } from "@/infrastructure/api";
import type { Novel, BookConfig, WriteResult, AuditResult, BookSource, SearchBookResult } from "@/shared/types";

export async function fetchNovels(): Promise<Novel[]> {
  return ipc<Novel[]>("list_novels");
}

export async function createNovelList(
  workspaceId: string,
  title: string,
  genre: string
): Promise<Novel> {
  return ipc<Novel>("create_novel", { workspaceId, title, genre });
}

export async function deleteNovel(id: string): Promise<boolean> {
  return ipc<boolean>("delete_novel", { id });
}

export async function createNovelPipeline(
  workspaceId: string,
  title: string,
  genre: string,
  brief?: string,
  targetChapters?: number,
  chapterWords?: number
): Promise<BookConfig> {
  return ipc<BookConfig>("novel_create", {
    workspaceId,
    title,
    genre,
    brief,
    targetChapters,
    chapterWords,
  });
}

export async function writeNextChapter(
  workspaceId: string,
  bookId: string,
  targetWords?: number
): Promise<WriteResult> {
  return ipc<WriteResult>("novel_write_next", { workspaceId, bookId, targetWords });
}

export async function planChapter(
  workspaceId: string,
  bookId: string,
  context?: string
): Promise<Record<string, unknown>> {
  return ipc<Record<string, unknown>>("novel_plan", { workspaceId, bookId, context });
}

export async function auditChapter(
  workspaceId: string,
  bookId: string,
  chapterNumber: number
): Promise<AuditResult> {
  return ipc<AuditResult>("novel_audit", { workspaceId, bookId, chapterNumber });
}

export async function reviseChapter(
  workspaceId: string,
  bookId: string,
  chapterNumber: number
): Promise<string> {
  return ipc<string>("novel_revise", { workspaceId, bookId, chapterNumber });
}

export async function observeChapter(
  workspaceId: string,
  bookId: string,
  chapterNumber: number
): Promise<Record<string, unknown>> {
  return ipc<Record<string, unknown>>("novel_observe", { workspaceId, bookId, chapterNumber });
}

export async function reflectChapter(
  workspaceId: string,
  bookId: string,
  chapterNumber: number
): Promise<Record<string, unknown>> {
  return ipc<Record<string, unknown>>("novel_reflect", { workspaceId, bookId, chapterNumber });
}

// ── Book Source (Novel Download) ──────────────────────

export async function listBookSources(): Promise<BookSource[]> {
  return ipc<BookSource[]>("novel_source_list");
}

export async function searchNovels(
  sourceName: string,
  keyword: string
): Promise<SearchBookResult[]> {
  return ipc<SearchBookResult[]>("novel_search", { sourceName, keyword });
}

export async function downloadNovel(
  sourceName: string,
  bookUrl: string,
  bookName: string
): Promise<string> {
  return ipc<string>("novel_download", { sourceName, bookUrl, bookName });
}

export async function listLocalNovels(): Promise<string[]> {
  return ipc<string[]>("novel_list_local");
}
