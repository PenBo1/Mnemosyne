import { create } from "zustand";

interface NovelReaderState {
  openNovelId: string | null;
  openNovelTitle: string;
  setOpenNovel: (id: string, title: string) => void;
  closeNovel: () => void;
}

export const useNovelReaderStore = create<NovelReaderState>((set) => ({
  openNovelId: null,
  openNovelTitle: "",
  setOpenNovel: (id, title) => set({ openNovelId: id, openNovelTitle: title }),
  closeNovel: () => set({ openNovelId: null, openNovelTitle: "" }),
}));