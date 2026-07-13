// ── Prompt ─────────────────────────────────────────────────

export interface Prompt {
  id: string;
  name: string;
  content: string;
  category: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}
