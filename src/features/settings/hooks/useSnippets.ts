import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  loadSnippets,
  saveSnippets,
  newSnippetId,
  type Snippet,
} from "@/services/settings";

/** 提示词片段 CRUD：持久化到 config.json */
export function useSnippets() {
  const { t } = useI18n();
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadSnippets().then((list) => {
      if (cancelled) return;
      setSnippets(list);
      setLoaded(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const persist = useCallback(async (list: Snippet[]) => {
    try {
      await saveSnippets(list);
      setSnippets(list);
    } catch (e) {
      console.error("[snippets] persist failed", e);
      toast.error(t.common.failedToSave);
    }
  }, []);

  // 新建或更新（按 id 合并）
  const upsert = useCallback(async (snippet: Snippet) => {
    const idx = snippets.findIndex((s) => s.id === snippet.id);
    const next = idx >= 0
      ? snippets.map((s) => (s.id === snippet.id ? snippet : s))
      : [...snippets, snippet];
    await persist(next);
    toast.success(t.common.updatedSuccessfully);
  }, [snippets, persist]);

  const remove = useCallback(async (id: string) => {
    await persist(snippets.filter((s) => s.id !== id));
    toast.success(t.common.deletedSuccessfully);
  }, [snippets, persist]);

  return { snippets, loaded, upsert, remove, newId: newSnippetId };
}
