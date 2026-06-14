import { useState, useEffect, useCallback } from "react";
import type { RadarScan } from "@/types";
import * as radarService from "@/services/radar";

export function useRadar() {
  const [currentResult, setCurrentResult] = useState<RadarScan | null>(null);
  const [history, setHistory] = useState<RadarScan[]>([]);
  const [scanning, setScanning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadHistory = useCallback(async () => {
    try {
      setLoading(true);
      const scans = await radarService.fetchRadarHistory();
      setHistory(scans);
    } catch (err) {
      console.error("Failed to load radar history:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const scan = useCallback(async () => {
    try {
      setScanning(true);
      setError(null);
      const result = await radarService.scanRadar();
      setCurrentResult(result);
      await loadHistory();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Scan failed";
      setError(message);
    } finally {
      setScanning(false);
    }
  }, [loadHistory]);

  const remove = useCallback(async (id: string) => {
    await radarService.deleteRadarScan(id);
    setHistory((prev) => prev.filter((s) => s.id !== id));
    if (currentResult?.id === id) {
      setCurrentResult(null);
    }
  }, [currentResult]);

  const viewHistoryItem = useCallback((scan: RadarScan) => {
    setCurrentResult(scan);
  }, []);

  return {
    currentResult,
    history,
    scanning,
    loading,
    error,
    scan,
    remove,
    viewHistoryItem,
  };
}
