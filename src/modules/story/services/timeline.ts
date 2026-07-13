import { ipc } from "@/infrastructure/api";
import type { TimelineEvent, TimelineEventType } from "@/shared/types";

export async function listTimelineEvents(novelId: string): Promise<TimelineEvent[]> {
  return ipc<TimelineEvent[]>("timeline_event_list", { novelId });
}

export async function createTimelineEvent(params: {
  novelId: string;
  title: string;
  description: string;
  event_date: string;
  event_type: TimelineEventType;
  chapter_number: number | null;
  tags: string[];
  sort_order: number;
  character_ids: string[];
}): Promise<TimelineEvent> {
  return ipc<TimelineEvent>("timeline_event_create", params);
}

export async function updateTimelineEvent(params: {
  id: string;
  title: string;
  description: string;
  event_date: string;
  event_type: TimelineEventType;
  chapter_number: number | null;
  tags: string[];
}): Promise<TimelineEvent> {
  return ipc<TimelineEvent>("timeline_event_update", params);
}

export async function deleteTimelineEvent(id: string): Promise<void> {
  await ipc<void>("timeline_event_delete", { id });
}
