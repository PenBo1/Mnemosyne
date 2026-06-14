import { ipc } from "@/lib/ipc";
import type { Prompt } from "@/types";

export async function fetchPrompts(category?: string): Promise<Prompt[]> {
  const cat = category === "all" ? undefined : category;
  return ipc<Prompt[]>("list_prompts", { category: cat });
}

export async function createPrompt(
  name: string,
  content: string,
  category: string,
  tags: string[]
): Promise<Prompt> {
  return ipc<Prompt>("create_prompt", { name, content, category, tags });
}

export async function updatePrompt(
  id: string,
  name: string,
  content: string,
  category: string
): Promise<Prompt> {
  return ipc<Prompt>("update_prompt", { id, name, content, category });
}

export async function deletePrompt(id: string): Promise<boolean> {
  return ipc<boolean>("delete_prompt", { id });
}
