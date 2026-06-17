import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { Novel, ChapterSummary, HookRecord, StoryFact } from "@/types";
import { ipc } from "@/lib/ipc";

interface StoryState {
  current_chapter: number;
  total_words: number;
  hooks: HookRecord[];
  summaries: ChapterSummary[];
  facts: StoryFact[];
}

export function useOverview(workspaceId: string | null) {
  const { t } = useI18n();
  const [novel, setNovel] = useState<Novel | null>(null);
  const [storyState, setStoryState] = useState<StoryState | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setNovel(null); setLoading(false); return; }
    try {
      setLoading(true);
      const novels = await ipc<Novel[]>("list_novels");
      const found = novels.find((n) => n.workspace_id === workspaceId);
      setNovel(found || null);
      if (found) {
        try {
          const state = await ipc<StoryState>("story_state_get", { novelId: found.id });
          setStoryState(state);
        } catch {
          setStoryState(null);
        }
      }
    } catch {
      setNovel(null);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

  useEffect(() => { load(); }, [load]);

  const updateNovel = useCallback(async (title: string, genre: string) => {
    if (!novel) return;
    try {
      const updated = await ipc<Novel>("update_novel", { id: novel.id, title, genre });
      setNovel(updated);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [novel, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  return { novel, storyState, loading, updateNovel, reload: load };
}
