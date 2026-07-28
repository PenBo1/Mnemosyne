import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { ResearchItem, ResearchCategory } from "@/features/story/types";
import { ipc } from "@/services/ipc";
import {
  listResearchItems,
  createResearchItem,
  updateResearchItem,
  deleteResearchItem,
} from "@/features/story/services";

export function useResearchItems(workspaceId: string | null) {
  const { t } = useI18n();
  const [items, setItems] = useState<ResearchItem[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setItems([]); setLoading(false); return; }
    try {
      setLoading(true);
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) { setItems([]); return; }
      const data = await listResearchItems(novel.id);
      setItems(data);
    } catch {
      setItems([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    title: string;
    content: string;
    category: ResearchCategory;
    tags: string[];
    source_url: string | null;
  }) => {
    if (!workspaceId) return;
    try {
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) return;
      await createResearchItem({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    content: string;
    category: ResearchCategory;
    tags: string[];
    source_url: string | null;
  }) => {
    try {
      await updateResearchItem(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load]);

  const remove = useCallback(async (id: string) => {
    try {
      await deleteResearchItem(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load]);

  return { items, loading, create, update, remove, reload: load };
}
