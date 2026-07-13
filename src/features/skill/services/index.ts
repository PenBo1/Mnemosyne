import { ipc } from "@/services/ipc";
import type { SkillMeta, Skill, CreateSkillParams, UpdateSkillParams } from "@/features/skill/types";

export async function listSkills(): Promise<SkillMeta[]> {
  return ipc<SkillMeta[]>("skill_list");
}

export async function getSkill(name: string): Promise<Skill> {
  return ipc<Skill>("skill_get", { name });
}

export async function createSkill(params: CreateSkillParams): Promise<Skill> {
  return ipc<Skill>("skill_create", { req: params });
}

export async function updateSkill(params: UpdateSkillParams): Promise<Skill> {
  return ipc<Skill>("skill_update", { req: params });
}

export async function deleteSkill(name: string): Promise<void> {
  return ipc<void>("skill_delete", { name });
}

export async function getSkillIndex(): Promise<string> {
  return ipc<string>("skill_index");
}

export async function refreshSkills(): Promise<number> {
  return ipc<number>("skill_refresh");
}
