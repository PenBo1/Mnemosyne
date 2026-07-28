import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { WorldSetting, WorldCategory } from "@/features/story/types";
import { ipc } from "@/services/ipc";
import {
  listWorldSettings,
  createWorldSetting,
  updateWorldSetting,
  deleteWorldSetting,
} from "@/features/story/services";

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
      const data = await listWorldSettings(novel.id);
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
      await createWorldSetting({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load]);

  const update = useCallback(async (params: {
    id: string;
    name: string;
    description: string;
    content: string;
    tags: string[];
  }) => {
    try {
      await updateWorldSetting(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load]);

  const remove = useCallback(async (id: string) => {
    try {
      await deleteWorldSetting(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load]);

  return { items, loading, create, update, remove, reload: load };
}
