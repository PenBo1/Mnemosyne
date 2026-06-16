import { useState, useEffect, useCallback } from "react";
import type { TimelineEvent, TimelineEventType } from "@/types";
import { ipc } from "@/lib/ipc";
import * as timelineService from "@/services/timeline";

export function useTimelineEvents(workspaceId: string | null) {
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setEvents([]); setLoading(false); return; }
    try {
      setLoading(true);
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) { setEvents([]); return; }
      const data = await timelineService.listTimelineEvents(novel.id);
      setEvents(data);
    } catch {
      setEvents([]);
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    title: string;
    description: string;
    event_date: string;
    event_type: TimelineEventType;
    chapter_number: number | null;
    tags: string[];
    sort_order: number;
    character_ids: string[];
  }) => {
    if (!workspaceId) return;
    const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novelList.find((n) => n.workspace_id === workspaceId);
    if (!novel) return;
    await timelineService.createTimelineEvent({ ...params, novelId: novel.id });
    await load();
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    description: string;
    event_date: string;
    event_type: TimelineEventType;
    chapter_number: number | null;
    tags: string[];
  }) => {
    await timelineService.updateTimelineEvent(params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await timelineService.deleteTimelineEvent(id);
    await load();
  }, [load]);

  return { events, loading, create, update, remove, reload: load };
}
