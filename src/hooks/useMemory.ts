import { useMemoryStore, MEMORY_CATEGORIES } from "@/stores/memory";

export function useMemory() {
  const {
    filterCategory,
    setFilterCategory,
    searchQuery,
    setSearchQuery,
    createMemory,
    updateMemory,
    removeMemory,
    getFilteredMemories,
  } = useMemoryStore();

  return {
    memories: getFilteredMemories(),
    filterCategory,
    setFilterCategory,
    searchQuery,
    setSearchQuery,
    create: createMemory,
    update: updateMemory,
    remove: removeMemory,
    categories: MEMORY_CATEGORIES,
  };
}
