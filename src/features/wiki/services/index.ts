import { ipc } from "@/infrastructure/api";
import type {
  WikiEntry,
  WikiGraphView,
  WikiEntityLink,
  CreateWikiEntryRequest,
  UpdateWikiEntryRequest,
  WikiCategory,
} from "@/shared/types";

// ── Wiki Entry CRUD ────────────────────────────────────────
//
// IPC 参数命名遵循项目约定：前端 camelCase，Rust snake_case，
// Tauri 自动转换。Rust 端 wiki_create_entry / wiki_update_entry
// 接收的是展开的扁平参数（而非 request 对象），这里展开传递。

export async function listWikiEntries(
  novelId: string,
  category?: WikiCategory
): Promise<WikiEntry[]> {
  return ipc<WikiEntry[]>("wiki_list_entries", { novelId, category });
}

export async function getWikiEntry(entryId: string): Promise<WikiEntry | null> {
  return ipc<WikiEntry | null>("wiki_get_entry", { entryId });
}

export async function createWikiEntry(
  novelId: string,
  request: CreateWikiEntryRequest
): Promise<WikiEntry> {
  return ipc<WikiEntry>("wiki_create_entry", {
    novelId,
    title: request.title,
    content: request.content,
    category: request.category,
    tags: request.tags ?? [],
    sourceChapter: request.source_chapter,
    importance: request.importance,
  });
}

export async function updateWikiEntry(
  entryId: string,
  request: UpdateWikiEntryRequest
): Promise<WikiEntry> {
  return ipc<WikiEntry>("wiki_update_entry", {
    entryId,
    title: request.title,
    content: request.content,
    category: request.category,
    tags: request.tags,
    importance: request.importance,
  });
}

export async function deleteWikiEntry(entryId: string): Promise<boolean> {
  return ipc<boolean>("wiki_delete_entry", { entryId });
}

// ── Wiki Graph ─────────────────────────────────────────────

export async function getWikiGraph(
  novelId: string,
  category?: WikiCategory,
  minImportance?: number
): Promise<WikiGraphView> {
  return ipc<WikiGraphView>("wiki_get_graph", { novelId, category, minImportance });
}

// ── Wiki Links ────────────────────────────────────────────

export async function createWikiLink(
  novelId: string,
  sourceEntryId: string,
  targetEntryId: string,
  relationType: string,
  relationDesc?: string,
  weight?: number,
  sourceChapter?: number
): Promise<WikiEntityLink> {
  return ipc<WikiEntityLink>("wiki_create_link", {
    novelId,
    sourceEntryId,
    targetEntryId,
    relationType,
    relationDesc: relationDesc ?? "",
    weight,
    sourceChapter,
  });
}

export async function deleteWikiLink(linkId: string): Promise<boolean> {
  return ipc<boolean>("wiki_delete_link", { linkId });
}

// ── Wiki Search ───────────────────────────────────────────

export async function searchWikiEntries(
  novelId: string,
  query: string,
  limit?: number
): Promise<WikiEntry[]> {
  return ipc<WikiEntry[]>("wiki_search", { novelId, query, limit });
}
