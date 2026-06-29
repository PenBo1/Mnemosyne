// ── Wiki Entry ─────────────────────────────────────────────

export type WikiCategory = "character" | "location" | "event" | "concept" | "item" | "other";

export interface WikiEntry {
  id: string;
  novel_id: string;
  title: string;
  content: string;
  category: WikiCategory;
  tags: string[];
  importance: number;
  source_chapter: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateWikiEntryRequest {
  title: string;
  content: string;
  category: WikiCategory;
  tags?: string[];
  importance?: number;
  source_chapter?: number;
}

export interface UpdateWikiEntryRequest {
  title?: string;
  content?: string;
  category?: WikiCategory;
  tags?: string[];
  importance?: number;
  source_chapter?: number;
}

export interface WikiEntityLink {
  id: string;
  novel_id: string;
  source_entry_id: string;
  target_entry_id: string;
  link_type: string;
  description: string;
  created_at: string;
}

export interface WikiGraphNode {
  id: string;
  title: string;
  category: WikiCategory;
  importance: number;
}

export interface WikiGraphLink {
  source: string;
  target: string;
  type: string;
}

export interface WikiGraphView {
  nodes: WikiGraphNode[];
  links: WikiGraphLink[];
}
