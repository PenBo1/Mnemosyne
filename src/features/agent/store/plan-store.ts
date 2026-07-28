// Plan Mode store。
//
// Plan mode 设计：
// - active=true 时，变更工具（write_file/edit/multi_edit/create_directory）
//   的 execute 只 enqueue 不真实写盘，返回值伪装成成功（让主 agent 继续）
// - 用户在 PlanDiffReview UI 审查队列，逐项 reject 或 Apply All
// - applyAll 顺序执行队列，逐项记录 ok/error，最后清空队列
//
// Mnemosyne 适配：
// - 写盘用 IPC（fs_write_file / fs_create_directory），不是 native.writeFile
// - create_directory 的 proposedContent 为空（只创建目录）

import { create } from "zustand";
import { writePlanFile, createPlanDirectory } from "@/features/agent/services/plan-persistence";

export type QueuedEdit = {
  id: string;
  kind: "write_file" | "edit" | "multi_edit" | "create_directory";
  path: string;
  /** 原文件内容（新文件为空）。用于 diff 预览。 */
  originalContent: string;
  /** 提议的完整内容（create_directory 为空）。 */
  proposedContent: string;
  isNewFile: boolean;
};

type PlanState = {
  active: boolean;
  queue: QueuedEdit[];
  toggle: () => void;
  enable: () => void;
  disable: () => void;
  enqueue: (q: QueuedEdit) => void;
  removeOne: (id: string) => void;
  clear: () => void;
  applyAll: () => Promise<{ id: string; ok: boolean; error?: string }[]>;
};

let nextId = 1;
export function newQueuedEditId(): string {
  return `q-${Date.now().toString(36)}-${(nextId++).toString(36)}`;
}

export const usePlanStore = create<PlanState>((set, get) => ({
  active: false,
  queue: [],
  toggle: () => set((s) => ({ active: !s.active, queue: s.active ? [] : s.queue })),
  enable: () => set({ active: true }),
  disable: () => set({ active: false, queue: [] }),
  enqueue: (q) => set((s) => ({ queue: [...s.queue, q] })),
  removeOne: (id) => set((s) => ({ queue: s.queue.filter((q) => q.id !== id) })),
  clear: () => set({ queue: [] }),
  async applyAll() {
    const items = get().queue;
    const results: { id: string; ok: boolean; error?: string }[] = [];
    const succeeded = new Set<string>();
    for (const q of items) {
      try {
        if (q.kind === "create_directory") {
          await createPlanDirectory(q.path);
        } else {
          await writePlanFile(q.path, q.proposedContent);
        }
        succeeded.add(q.id);
        results.push({ id: q.id, ok: true });
      } catch (e) {
        results.push({ id: q.id, ok: false, error: String(e) });
      }
    }
    // 只清空成功的项，失败项保留在队列供用户重试或排查
    set({ queue: items.filter((q) => !succeeded.has(q.id)) });
    return results;
  },
}));
