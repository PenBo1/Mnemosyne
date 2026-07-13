import { invoke } from "@tauri-apps/api/core";

export interface IpcResponse<T> {
  status: number;
  code: string;
  message: string;
  data: T | null;
}

const SUCCESS_CODES = new Set([0, 1, 2, 3, 4, 5]);

export async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const response = await invoke<IpcResponse<T>>(command, args);
  if (!SUCCESS_CODES.has(response.status)) {
    throw new Error(response.message || `IPC error [${response.status}|${response.code}]`);
  }
  if (response.data === null || response.data === undefined) {
    throw new Error(`IPC command "${command}" returned null data`);
  }
  return response.data;
}

export async function ipcVoid(command: string, args?: Record<string, unknown>): Promise<void> {
  const response = await invoke<IpcResponse<void>>(command, args);
  if (!SUCCESS_CODES.has(response.status)) {
    throw new Error(response.message || `IPC error [${response.status}|${response.code}]`);
  }
}

// ── IPC Batch Call Support (P2 from AI Engineering curriculum) ──────────────

interface BatchRequest {
  command: string;
  args?: Record<string, unknown>;
}

interface BatchResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Execute multiple IPC calls in parallel (P2 optimization)
 * Reduces latency by batching independent calls together
 */
export async function ipcBatch<T>(
  requests: BatchRequest[]
): Promise<BatchResult<T>[]> {
  const promises = requests.map(async (req) => {
    try {
      const data = await ipc<T>(req.command, req.args);
      return { success: true, data };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : "Unknown error",
      };
    }
  });

  return Promise.all(promises);
}

/**
 * Execute multiple IPC calls sequentially with early termination on failure
 * Use when order matters or dependencies exist between calls
 */
export async function ipcSequential<T>(
  requests: BatchRequest[],
  stopOnError = true
): Promise<BatchResult<T>[]> {
  const results: BatchResult<T>[] = [];

  for (const req of requests) {
    try {
      const data = await ipc<T>(req.command, req.args);
      results.push({ success: true, data });
    } catch (err) {
      const result: BatchResult<T> = {
        success: false,
        error: err instanceof Error ? err.message : "Unknown error",
      };
      results.push(result);
      if (stopOnError) break;
    }
  }

  return results;
}

/**
 * Cached IPC calls with TTL (Time-To-Live)
 * Prevents redundant calls for frequently accessed data
 */
const ipcCache = new Map<string, { data: unknown; expiry: number }>();

export async function ipcCached<T>(
  command: string,
  args?: Record<string, unknown>,
  ttlMs = 5000
): Promise<T> {
  const cacheKey = `${command}:${JSON.stringify(args || {})}`;
  const now = Date.now();

  const cached = ipcCache.get(cacheKey);
  if (cached && cached.expiry > now) {
    return cached.data as T;
  }

  const data = await ipc<T>(command, args);
  ipcCache.set(cacheKey, { data, expiry: now + ttlMs });
  return data;
}

/**
 * Clear IPC cache (call after mutations that affect cached data)
 */
export function clearIpcCache(pattern?: string): void {
  if (!pattern) {
    ipcCache.clear();
    return;
  }
  for (const key of ipcCache.keys()) {
    if (key.startsWith(pattern)) {
      ipcCache.delete(key);
    }
  }
}
