// ── Plot Point ────────────────────────────────────────────

export type PlotPointType = "act" | "chapter" | "scene";

export interface PlotPoint {
  id: string;
  novel_id: string;
  type: PlotPointType;
  parent_id: string | null;
  title: string;
  description: string;
  sort_order: number;
  chapter_number: number | null;
  pov_character_id: string | null;
  location_id: string | null;
  goals: string;
  conflicts: string;
  outcome: string;
  status: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}
