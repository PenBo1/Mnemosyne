import { useState, useEffect, useCallback } from "react";
import * as sandboxService from "@/services/sandbox";
import type { SandboxStatus } from "@/services/sandbox";

export function useSandboxStatus() {
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
      const message = err instanceof Error ? err.message : "Failed to load sandbox status";
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

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
