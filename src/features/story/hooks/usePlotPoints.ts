import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { PlotPoint, PlotPointType } from "@/features/story/types";
import { ipc } from "@/services/ipc";
import {
  listPlotPoints,
  createPlotPoint,
  updatePlotPoint,
  deletePlotPoint,
} from "@/features/story/services";

export function usePlotPoints(workspaceId: string | null) {
  const { t } = useI18n();
  const [points, setPoints] = useState<PlotPoint[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setPoints([]); setLoading(false); return; }
    try {
      setLoading(true);
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) { setPoints([]); return; }
      const data = await listPlotPoints(novel.id);
      setPoints(data);
    } catch {
      setPoints([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    type: PlotPointType;
    title: string;
    description: string;
    status: string;
    chapter_number: number | null;
    goals: string;
    conflicts: string;
    outcome: string;
    sort_order: number;
  }) => {
    if (!workspaceId) return;
    try {
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) return;
      await createPlotPoint({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    description: string;
    type: PlotPointType;
    status: string;
    chapter_number: number | null;
    goals: string;
    conflicts: string;
    outcome: string;
  }) => {
    try {
      await updatePlotPoint(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load]);

  const remove = useCallback(async (id: string) => {
    try {
      await deletePlotPoint(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load]);

  return { points, loading, create, update, remove, reload: load };
}
