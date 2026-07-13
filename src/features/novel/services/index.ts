import { ipc } from "@/services/ipc";
import type { Novel, BookSource, SearchBookResult } from "@/features/novel/types";

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
