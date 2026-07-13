// ── World Setting ─────────────────────────────────────────

export type WorldCategory =
  | "location"
  | "faction"
  | "species"
  | "culture"
  | "history"
  | "magic_system"
  | "language"
  | "architecture";

export interface WorldSetting {
  id: string;
  novel_id: string;
  category: WorldCategory;
  name: string;
  description: string;
  content: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}
