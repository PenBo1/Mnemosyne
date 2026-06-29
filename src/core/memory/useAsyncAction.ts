import { useState, useCallback } from "react";
import { toast } from "sonner";

/**
 * 共享 loading/error 的异步操作 hook。
 *
 * 统一替代此前在各 hook 中反复手写的
 * `setLoading / try / catch / toast / finally` 三件套。
 *
 * 多任务场景：同一 hook 内多个方法共享一个 loading/error，
 * 调用方多次调用本 hook 即可（满足 React hooks 规则）。
 *
 * 用法：
 * ```ts
 * const { loading, error, run } = useAsyncAction();
 * const create = async (req: Req) => {
 *   const result = await run(
 *     () => service.create(req),
 *     { successToast: t.common.createdSuccessfully, errorToast: t.common.failedToCreate },
 *   );
 *   if (result) setData(result);
 *   return result;
 * };
 * ```
 */
export function useAsyncAction() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async <T,>(
    fn: () => Promise<T>,
    opts?: {
      successToast?: string;
      errorToast?: string;
      onSuccess?: (data: T) => void;
    },
  ): Promise<T | null> => {
    setLoading(true);
    setError(null);
    try {
      const result = await fn();
      if (opts?.successToast) toast.success(opts.successToast);
      opts?.onSuccess?.(result);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : (opts?.errorToast ?? "Unknown error");
      setError(msg);
      toast.error(msg);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  return { loading, error, run };
}
