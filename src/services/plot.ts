import { ipc } from "@/lib/ipc";
import type { PlotPoint, PlotPointType } from "@/types";

export async function listPlotPoints(novelId: string): Promise<PlotPoint[]> {
  return ipc<PlotPoint[]>("plot_point_list", { novelId });
}

export async function createPlotPoint(params: {
  novelId: string;
  type: PlotPointType;
  title: string;
  description: string;
  status: string;
  chapter_number: number | null;
  goals: string;
  conflicts: string;
  outcome: string;
  sort_order: number;
}): Promise<PlotPoint> {
  return ipc<PlotPoint>("plot_point_create", params);
}

export async function updatePlotPoint(params: {
  id: string;
  title: string;
  description: string;
  type: PlotPointType;
  status: string;
  chapter_number: number | null;
  goals: string;
  conflicts: string;
  outcome: string;
}): Promise<PlotPoint> {
  return ipc<PlotPoint>("plot_point_update", params);
}

export async function deletePlotPoint(id: string): Promise<void> {
  await ipc<void>("plot_point_delete", { id });
}
