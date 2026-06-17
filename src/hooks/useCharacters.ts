import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import type { Character, CharacterRelationship } from "@/types";
import { ipc } from "@/lib/ipc";
import * as characterService from "@/services/character";

export function useCharacters(workspaceId: string | null) {
  const { t } = useI18n();
  const [characters, setCharacters] = useState<Character[]>([]);
  const [relationships, setRelationships] = useState<CharacterRelationship[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!workspaceId) { setCharacters([]); setRelationships([]); setLoading(false); return; }
    try {
      setLoading(true);
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) { setCharacters([]); setRelationships([]); return; }
      const [chars, rels] = await Promise.all([
        characterService.listCharacters(novel.id),
        characterService.listCharacterRelationships(novel.id),
      ]);
      setCharacters(chars);
      setRelationships(rels);
    } catch {
      setCharacters([]);
      setRelationships([]);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, t.common.failedToLoad]);

  useEffect(() => { load(); }, [load]);

  const create = useCallback(async (params: {
    name: string;
    role: string;
    age: string;
    gender: string;
    appearance: string;
    personality: string;
    backstory: string;
    motivation: string;
    fears: string;
    skills: string;
    description: string;
    traits: string[];
  }) => {
    if (!workspaceId) return;
    try {
      const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novelList.find((n) => n.workspace_id === workspaceId);
      if (!novel) return;
      await characterService.createCharacter({ ...params, novelId: novel.id });
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [workspaceId, load, t.common.createdSuccessfully, t.common.failedToCreate]);

  const update = useCallback(async (params: {
    id: string;
    name: string;
    role: string;
    age: string;
    gender: string;
    appearance: string;
    personality: string;
    backstory: string;
    motivation: string;
    fears: string;
    skills: string;
    description: string;
    traits: string[];
  }) => {
    try {
      await characterService.updateCharacter(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load, t.common.updatedSuccessfully, t.common.failedToUpdate]);

  const remove = useCallback(async (id: string) => {
    try {
      await characterService.deleteCharacter(id);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [load, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return { characters, relationships, loading, create, update, remove, reload: load };
}
