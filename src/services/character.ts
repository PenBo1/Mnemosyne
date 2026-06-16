import { ipc } from "@/lib/ipc";
import type { Character, CharacterRelationship } from "@/types";

export async function listCharacters(novelId: string): Promise<Character[]> {
  return ipc<Character[]>("character_list", { novelId });
}

export async function listCharacterRelationships(novelId: string): Promise<CharacterRelationship[]> {
  return ipc<CharacterRelationship[]>("character_relationship_list", { novelId }).catch(() => []);
}

export async function createCharacter(params: {
  novelId: string;
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
}): Promise<Character> {
  return ipc<Character>("character_create", params);
}

export async function updateCharacter(params: {
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
}): Promise<Character> {
  return ipc<Character>("character_update", params);
}

export async function deleteCharacter(id: string): Promise<void> {
  await ipc<void>("character_delete", { id });
}
