// ── Research Item ─────────────────────────────────────────

export type ResearchCategory = "reference" | "inspiration" | "note" | "link";

export interface ResearchItem {
  id: string;
  novel_id: string;
  title: string;
  content: string;
  category: ResearchCategory;
  tags: string[];
  source_url: string | null;
  created_at: string;
  updated_at: string;
}
