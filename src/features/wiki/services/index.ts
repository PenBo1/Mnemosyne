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
  return ipc<WikiEntry>("wiki_create_entry", { novelId, request });
}

export async function updateWikiEntry(
  entryId: string,
  request: UpdateWikiEntryRequest
): Promise<WikiEntry> {
  return ipc<WikiEntry>("wiki_update_entry", { entryId, request });
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
  sourceId: string,
  targetId: string,
  linkType: string,
  description?: string
): Promise<WikiEntityLink> {
  return ipc<WikiEntityLink>("wiki_create_link", {
    novelId,
    sourceId,
    targetId,
    linkType,
    description,
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