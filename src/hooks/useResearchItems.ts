import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { ResearchItem, ResearchCategory } from "@/types";
import { ipc } from "@/lib/ipc";
import * as researchService from "@/services/research";

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
      const data = await researchService.listResearchItems(novel.id);
      setItems(data);
    } catch {
      setItems([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

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
      await researchService.createResearchItem({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load, t.common.createdSuccessfully, t.common.failedToCreate]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    content: string;
    category: ResearchCategory;
    tags: string[];
    source_url: string | null;
  }) => {
    try {
      await researchService.updateResearchItem(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await researchService.deleteResearchItem(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return { items, loading, create, update, remove, reload: load };
}
