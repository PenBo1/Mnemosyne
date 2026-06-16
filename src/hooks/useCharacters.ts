import { useState, useEffect, useCallback } from "react";
import type { Character, CharacterRelationship } from "@/types";
import { ipc } from "@/lib/ipc";
import * as characterService from "@/services/character";

export function useCharacters(workspaceId: string | null) {
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
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

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
    const novelList = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novelList.find((n) => n.workspace_id === workspaceId);
    if (!novel) return;
    await characterService.createCharacter({ ...params, novelId: novel.id });
    await load();
  }, [workspaceId, load]);

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
    await characterService.updateCharacter(params);
    await load();
  }, [load]);

  const remove = useCallback(async (id: string) => {
    await characterService.deleteCharacter(id);
    await load();
  }, [load]);

  return { characters, relationships, loading, create, update, remove, reload: load };
}
