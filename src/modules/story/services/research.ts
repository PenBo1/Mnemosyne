import { ipc } from "@/infrastructure/api";
import type { ResearchItem, ResearchCategory } from "@/shared/types";

export async function listResearchItems(novelId: string): Promise<ResearchItem[]> {
  return ipc<ResearchItem[]>("research_item_list", { novelId });
}

export async function createResearchItem(params: {
  novelId: string;
  title: string;
  content: string;
  category: ResearchCategory;
  tags: string[];
  source_url: string | null;
}): Promise<ResearchItem> {
  return ipc<ResearchItem>("research_item_create", params);
}

export async function updateResearchItem(params: {
  id: string;
  title: string;
  content: string;
  category: ResearchCategory;
  tags: string[];
  source_url: string | null;
}): Promise<ResearchItem> {
  return ipc<ResearchItem>("research_item_update", params);
}

export async function deleteResearchItem(id: string): Promise<void> {
  await ipc<void>("research_item_delete", { id });
}
