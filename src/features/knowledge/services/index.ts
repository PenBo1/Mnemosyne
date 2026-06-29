import { LazyStore } from "@tauri-apps/plugin-store";
import type { KnowledgeEntry } from "@/shared/types";

const STORE_KEY = "knowledge-entries";

const store = new LazyStore("config.json", {
  defaults: {} as Record<string, unknown>,
  autoSave: 100,
});

export async function loadEntries(): Promise<KnowledgeEntry[]> {
  const entries = await store.get<KnowledgeEntry[]>(STORE_KEY);
  return entries || [];
}

export async function saveEntries(entries: KnowledgeEntry[]): Promise<void> {
  await store.set(STORE_KEY, entries);
  await store.save();
}

export async function createEntry(entry: Omit<KnowledgeEntry, "id" | "created_at" | "updated_at">): Promise<KnowledgeEntry> {
  const entries = await loadEntries();
  const now = new Date().toISOString();
  const newEntry: KnowledgeEntry = {
    ...entry,
    id: crypto.randomUUID(),
    created_at: now,
    updated_at: now,
  };
  await saveEntries([newEntry, ...entries]);
  return newEntry;
}

export async function updateEntry(id: string, updates: Partial<Omit<KnowledgeEntry, "id" | "created_at">>): Promise<void> {
  const entries = await loadEntries();
  const now = new Date().toISOString();
  const updated = entries.map((e) =>
    e.id === id ? { ...e, ...updates, updated_at: now } : e
  );
  await saveEntries(updated);
}

export async function deleteEntry(id: string): Promise<void> {
  const entries = await loadEntries();
  await saveEntries(entries.filter((e) => e.id !== id));
}
