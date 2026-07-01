import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import { ipc } from "@/infrastructure/api";
import type { HookStatus, StoryState } from "@/shared/types";

interface UseHookLedgerResult {
  loading: boolean;
  storyState: StoryState | null;
  updatingHookId: string | null;
  reload: () => Promise<void>;
  updateHookStatus: (hookId: string, newStatus: HookStatus) => Promise<void>;
}

/**
 * Hook 账本管理：加载 StoryState（hooks/summaries/facts）+ 手动更新 hook 状态。
 *
 * - `story_state_get`：从 `<workspace>/books/<book_id>/story/state.json` 读取
 * - `hook_update_status`：手动 resolve/defer/reopen hook，写回 state.json
 */
export function useHookLedger(novelId: string | null): UseHookLedgerResult {
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [storyState, setStoryState] = useState<StoryState | null>(null);
  const [updatingHookId, setUpdatingHookId] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!novelId) {
      setStoryState(null);
      return;
    }
    setLoading(true);
    try {
      const state = await ipc<StoryState>("story_state_get", { novelId });
      setStoryState(state);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load hook ledger";
      toast.error(msg);
      setStoryState(null);
    } finally {
      setLoading(false);
    }
  }, [novelId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const updateHookStatus = useCallback(
    async (hookId: string, newStatus: HookStatus) => {
      if (!novelId) return;
      setUpdatingHookId(hookId);
      try {
        const updated = await ipc<StoryState>("hook_update_status", {
          novelId,
          hookId,
          newStatus,
        });
        setStoryState(updated);
        toast.success(t.common.updatedSuccessfully);
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to update hook";
        toast.error(msg);
      } finally {
        setUpdatingHookId(null);
      }
    },
    [novelId, t.common.updatedSuccessfully]
  );

  // 仅暴露 hooks（其他字段由 useOverview 管理）
  return {
    loading,
    storyState,
    updatingHookId,
    reload,
    updateHookStatus,
  };
}
