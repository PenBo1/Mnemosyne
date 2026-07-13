import { ipc } from "@/services/ipc";
import type { RadarScan } from "@/features/radar/types";

export async function scanRadar(): Promise<RadarScan> {
  return ipc<RadarScan>("radar_scan");
}

export async function fetchRadarHistory(limit?: number): Promise<RadarScan[]> {
  return ipc<RadarScan[]>("radar_scan_list", { limit });
}

export async function deleteRadarScan(id: string): Promise<boolean> {
  return ipc<boolean>("radar_scan_delete", { scanId: id });
}
