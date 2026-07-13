// ── Timeline Event ────────────────────────────────────────

export type TimelineEventType = "event" | "milestone" | "turning_point";

export interface TimelineEvent {
  id: string;
  novel_id: string;
  title: string;
  description: string;
  event_date: string;
  sort_order: number;
  chapter_number: number | null;
  character_ids: string[];
  location_id: string | null;
  event_type: TimelineEventType;
  tags: string[];
  created_at: string;
  updated_at: string;
}
