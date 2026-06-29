import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import {
  getLlmCalls,
  getToolExecutions,
  getTokenUsage,
  getSandboxViolations,
} from "@/features/stats/services";
import type {
  LlmCall,
  ToolExecution,
  TokenUsageStats,
  SandboxViolation,
} from "@/shared/types";

export interface AiAnalyticsData {
  llmCalls: LlmCall[];
  toolExecutions: ToolExecution[];
  tokenUsage: TokenUsageStats | null;
  sandboxViolations: SandboxViolation[];
}

export function useAiAnalytics(sessionId: string | null) {
  const [data, setData] = useState<AiAnalyticsData>({
    llmCalls: [],
    toolExecutions: [],
    tokenUsage: null,
    sandboxViolations: [],
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const [llmCalls, toolExecutions, tokenUsage, sandboxViolations] =
        await Promise.all([
          getLlmCalls(sessionId, 100),
          getToolExecutions(sessionId, 100),
          getTokenUsage(sessionId),
          getSandboxViolations(sessionId, 50),
        ]);
      setData({ llmCalls, toolExecutions, tokenUsage, sandboxViolations });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load AI analytics";
      setError(msg);
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    load();
  }, [load]);

  return { ...data, loading, error, reload: load };
}
