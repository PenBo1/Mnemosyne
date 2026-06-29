import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import type { PlotPoint, PlotPointType } from "@/shared/types";
import { ipc } from "@/infrastructure/api";
import * as plotService from "@/features/story/services";

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
      const data = await plotService.listPlotPoints(novel.id);
      setPoints(data);
    } catch {
      setPoints([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

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
      await plotService.createPlotPoint({ ...params, novelId: novel.id });
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
    type: PlotPointType;
    status: string;
    chapter_number: number | null;
    goals: string;
    conflicts: string;
    outcome: string;
  }) => {
    try {
      await plotService.updatePlotPoint(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await plotService.deletePlotPoint(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return { points, loading, create, update, remove, reload: load };
}
