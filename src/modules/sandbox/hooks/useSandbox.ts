import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import * as sandboxService from "@/features/sandbox/services";
import type { SandboxStatus } from "@/shared/types";

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
  }, [t.common.failedToLoad]);

  useEffect(() => {
    load();
  }, [load]);

  return { status, loading, error, reload: load };
}

export function useSandboxValidation() {
  const [validating, setValidating] = useState(false);

  const validateFile = useCallback(async (path: string, isWrite: boolean) => {
    setValidating(true);
    try {
      return await sandboxService.validateFile(path, isWrite);
    } finally {
      setValidating(false);
    }
  }, []);

  const validateCommand = useCallback(async (command: string) => {
    setValidating(true);
    try {
      return await sandboxService.validateCommand(command);
    } finally {
      setValidating(false);
    }
  }, []);

  const validateNetwork = useCallback(async (url: string) => {
    setValidating(true);
    try {
      return await sandboxService.validateNetwork(url);
    } finally {
      setValidating(false);
    }
  }, []);

  return { validating, validateFile, validateCommand, validateNetwork };
}
