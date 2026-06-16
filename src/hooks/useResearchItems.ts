import { useState, useEffect, useCallback } from "react";
import type { ResearchItem, ResearchCategory } from "@/types";
import { ipc } from "@/lib/ipc";
import * as researchService from "@/services/research";

export function useResearchItems(workspaceId: string | null) {
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
    const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novelList.find((n) => n.workspace_id === workspaceId);
    if (!novel) return;
    await researchService.createResearchItem({ ...params, novelId: novel.id });
    await load();
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    title: string;
    content: string;
    category: ResearchCategory;
    tags: string[];
    source_url: string | null;
  }) => {
    await researchService.updateResearchItem(params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await researchService.deleteResearchItem(id);
    await load();
  }, [load]);

  return { items, loading, create, update, remove, reload: load };
}
