import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { getStats, getDailyActivity } from "@/services/stats";
import type { StatsData, DailyActivity } from "@/services/stats";

export function useDashboard() {
  const [stats, setStats] = useState<StatsData | null>(null);
  const [activity, setActivity] = useState<DailyActivity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [statsData, activityData] = await Promise.all([
        getStats(),
        getDailyActivity(),
      ]);
      setStats(statsData);
      setActivity(activityData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load data");
      toast.error(err instanceof Error ? err.message : "Failed to load data");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  return { stats, activity, loading, error, reload: load };
}
