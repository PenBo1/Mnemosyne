import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { WorldSetting, WorldCategory } from "@/types";
import { ipc } from "@/lib/ipc";
import * as worldService from "@/services/world";

export function useWorldSettings(workspaceId: string | null) {
  const { t } = useI18n();
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
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    category: WorldCategory;
    name: string;
    description: string;
    content: string;
    tags: string[];
  }) => {
    if (!workspaceId) return;
    try {
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) return;
      await worldService.createWorldSetting({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load, t.common.createdSuccessfully, t.common.failedToCreate]);

  const update = useCallback(async (params: {
    id: string;
    name: string;
    description: string;
    content: string;
    tags: string[];
  }) => {
    try {
      await worldService.updateWorldSetting(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await worldService.deleteWorldSetting(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return { items, loading, create, update, remove, reload: load };
}
