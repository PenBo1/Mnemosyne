import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import * as sandboxService from "@/features/sandbox/services";
import type { SandboxStatus } from "@/features/sandbox/types";

export function useSandboxStatus() {
  const { t } = useI18n();
  const [status, setStatus] = useState<SandboxStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await sandboxService.getSandboxStatus();
      setStatus(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : t.common.failedToLoad;
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return { status, loading, error, reload: load };
}
