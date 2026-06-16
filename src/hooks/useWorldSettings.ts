import { useState, useEffect, useCallback } from "react";
import type { WorldSetting, WorldCategory } from "@/types";
import { ipc } from "@/lib/ipc";
import * as worldService from "@/services/world";

export function useWorldSettings(workspaceId: string | null) {
  const [items, setItems] = useState<WorldSetting[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setItems([]); setLoading(false); return; }
    try {
      setLoading(true);
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) { setItems([]); return; }
      const data = await worldService.listWorldSettings(novel.id);
      setItems(data);
    } catch {
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    category: WorldCategory;
    name: string;
    description: string;
    content: string;
    tags: string[];
  }) => {
    if (!workspaceId) return;
    const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novelList.find((n) => n.workspace_id === workspaceId);
    if (!novel) return;
    await worldService.createWorldSetting({ ...params, novelId: novel.id });
    await load();
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    name: string;
    description: string;
    content: string;
    tags: string[];
  }) => {
    await worldService.updateWorldSetting(params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await worldService.deleteWorldSetting(id);
    await load();
  }, [load]);

  return { items, loading, create, update, remove, reload: load };
}
