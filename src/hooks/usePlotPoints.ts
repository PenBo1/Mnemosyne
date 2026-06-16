import { useState, useEffect, useCallback } from "react";
import type { PlotPoint, PlotPointType } from "@/types";
import { ipc } from "@/lib/ipc";
import * as plotService from "@/services/plot";

export function usePlotPoints(workspaceId: string | null) {
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
    const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novelList.find((n) => n.workspace_id === workspaceId);
    if (!novel) return;
    await plotService.createPlotPoint({ ...params, novelId: novel.id });
    await load();
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
    await plotService.updatePlotPoint(params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await plotService.deletePlotPoint(id);
    await load();
  }, [load]);

  return { points, loading, create, update, remove, reload: load };
}
