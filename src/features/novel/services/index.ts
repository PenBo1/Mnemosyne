import { ipc } from "@/services/ipc";
import type { Novel, BookSource, SearchBookResult } from "@/features/novel/types";

export async function fetchNovels(): Promise<Novel[]> {
  return ipc<Novel[]>("novel_list");
}

export async function createNovelList(
  workspaceId: string,
  title: string,
  genre: string
): Promise<Novel> {
  return ipc<Novel>("novel_create", { workspaceId, title, genre });
}

export async function deleteNovel(id: string): Promise<boolean> {
  return ipc<boolean>("novel_delete", { id });
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

// 后端返回 LocalBookItem(name/size/timestamp),前端 UI 只需要文件名
interface LocalBookItem {
  name: string;
  size: number;
  timestamp: number;
}

export async function listLocalNovels(): Promise<string[]> {
  const items = await ipc<LocalBookItem[]>("novel_list_local");
  return items.map((item) => item.name);
}
