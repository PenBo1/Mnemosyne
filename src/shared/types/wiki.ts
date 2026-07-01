// ── Wiki Entry ─────────────────────────────────────────────
//
// 字段与 src-tauri/src/features/wiki/models.rs 对齐，
// WikiCategory 取值与 src-tauri/src/shared/wiki.rs + DB CHECK 约束对齐：
//   general / character / location / event / concept / reference

export type WikiCategory = "general" | "character" | "location" | "event" | "concept" | "reference";

export type WikiSourceType = "manual" | "ai_extracted" | "imported";

export interface WikiEntry {
  id: string;
  novel_id: string;
  title: string;
  content: string;
  category: WikiCategory;
  source_type: WikiSourceType;
  source_chapter: number | null;
  tags: string[];
  importance: number;
  word_count: number;
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
  relation_type: string;
  relation_desc: string;
  weight: number;
  source_chapter: number | null;
  created_at: string;
}

export interface WikiGraphNode {
  id: string;
  title: string;
  category: WikiCategory;
  importance: number;
}

export interface WikiGraphEdge {
  source: string;
  target: string;
  relation: string;
  weight: number;
}

export interface WikiGraphView {
  nodes: WikiGraphNode[];
  edges: WikiGraphEdge[];
}
