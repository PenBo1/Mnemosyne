import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import type { TimelineEvent, TimelineEventType } from "@/shared/types";
import { ipc } from "@/infrastructure/api";
import * as timelineService from "@/features/story/services";

export function useTimelineEvents(workspaceId: string | null) {
  const { t } = useI18n();
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
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

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
    try {
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) return;
      await timelineService.createTimelineEvent({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load, t.common.createdSuccessfully, t.common.failedToCreate]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    description: string;
    event_date: string;
    event_type: TimelineEventType;
    chapter_number: number | null;
    tags: string[];
  }) => {
    try {
      await timelineService.updateTimelineEvent(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await timelineService.deleteTimelineEvent(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return { events, loading, create, update, remove, reload: load };
}
