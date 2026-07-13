import { ipc } from "@/infrastructure/api";
import type { RadarScan } from "@/shared/types";

export async function scanRadar(): Promise<RadarScan> {
  return ipc<RadarScan>("radar_scan");
}

export async function fetchRadarHistory(limit?: number): Promise<RadarScan[]> {
  return ipc<RadarScan[]>("radar_history", { limit });
}

export async function deleteRadarScan(id: string): Promise<boolean> {
  return ipc<boolean>("radar_delete", { id });
}
