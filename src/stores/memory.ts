import { create } from "zustand";
import type { Memory } from "@/types";
import { MEMORY_CATEGORIES } from "@/types";

interface MemoryState {
  memories: Memory[];
  filterCategory: string;
  searchQuery: string;
  loading: boolean;
  error: string | null;
  setFilterCategory: (category: string) => void;
  setSearchQuery: (query: string) => void;
  loadMemories: () => void;
  createMemory: (memory: Omit<Memory, "id" | "created_at" | "updated_at">) => Memory;
  updateMemory: (id: string, updates: Partial<Memory>) => void;
  removeMemory: (id: string) => void;
  getFilteredMemories: () => Memory[];
}

const INITIAL_MEMORIES: Memory[] = [
  {
    id: "1",
    title: "Main Character Profile",
    content: "Name: Elara Nightshade. Age: 25. A skilled alchemist with a mysterious past...",
    category: "character",
    tags: ["protagonist", "alchemy", "mystery"],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "2",
    title: "World Building - The Academy",
    content: "The Arcane Academy sits atop Crystal Peak...",
    category: "world",
    tags: ["academy", "magic", "setting"],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "3",
    title: "Plot Outline - Act 1",
    content: "Chapter 1: Elara discovers her alchemical abilities...",
    category: "plot",
    tags: ["outline", "act-1", "beginning"],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

export const useMemoryStore = create<MemoryState>((set, get) => ({
  memories: INITIAL_MEMORIES,
  filterCategory: "all",
  searchQuery: "",
  loading: false,
  error: null,

  setFilterCategory: (category) => set({ filterCategory: category }),
  setSearchQuery: (query) => set({ searchQuery: query }),

  loadMemories: () => {
    // TODO: Replace with IPC call when backend memory commands are implemented
    set({ memories: INITIAL_MEMORIES, loading: false });
  },

  createMemory: (memory) => {
    const newMemory: Memory = {
      ...memory,
      id: crypto.randomUUID(),
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    set((state) => ({ memories: [...state.memories, newMemory] }));
    return newMemory;
  },

  updateMemory: (id, updates) => {
    set((state) => ({
      memories: state.memories.map((m) =>
        m.id === id ? { ...m, ...updates, updated_at: new Date().toISOString() } : m
      ),
    }));
  },

  removeMemory: (id) => {
    set((state) => ({
      memories: state.memories.filter((m) => m.id !== id),
    }));
  },

  getFilteredMemories: () => {
    const { memories, filterCategory, searchQuery } = get();
    return memories.filter((memory) => {
      const matchesCategory = filterCategory === "all" || memory.category === filterCategory;
      const matchesSearch =
        memory.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        memory.content.toLowerCase().includes(searchQuery.toLowerCase()) ||
        memory.tags.some((tag) => tag.toLowerCase().includes(searchQuery.toLowerCase()));
      return matchesCategory && matchesSearch;
    });
  },
}));

export { MEMORY_CATEGORIES };
