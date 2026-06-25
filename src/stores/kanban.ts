import { create } from "zustand";
import type { KanbanTask, KanbanState, CreateKanbanTaskRequest, UpdateKanbanTaskRequest, KanbanTaskStatus } from "@/types";
import * as kanbanService from "@/services/kanban";
import { toast } from "sonner";

export const useKanbanStore = create<KanbanState>((set, _get) => ({
  tasks: [],
  columns: [],
  loading: false,
  error: null,

  loadTasks: async (novelId: string) => {
    set({ loading: true, error: null });
    try {
      const tasks = await kanbanService.getTasks(novelId);
      set({ tasks, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load tasks";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  loadColumns: async (novelId: string) => {
    try {
      const columns = await kanbanService.getColumns(novelId);
      set({ columns });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load columns";
      toast.error(message);
    }
  },

  createTask: async (novelId: string, req: CreateKanbanTaskRequest) => {
    const tempId = `temp-${Date.now()}`;
    const optimistic: KanbanTask = {
      id: tempId,
      novel_id: novelId,
      title: req.title,
      description: req.description ?? "",
      status: req.status ?? "plan",
      priority: req.priority ?? "medium",
      assigned_agent: req.assigned_agent ?? null,
      chapter_id: req.chapter_id ?? null,
      parent_task_id: req.parent_task_id ?? null,
      tags: req.tags ?? [],
      sort_order: 0,
      due_date: req.due_date ?? null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    set((state) => ({ tasks: [...state.tasks, optimistic] }));

    try {
      const task = await kanbanService.createTask(novelId, req);
      set((state) => ({
        tasks: state.tasks.map((t) => (t.id === tempId ? task : t)),
      }));
      return task;
    } catch (err) {
      set((state) => ({
        tasks: state.tasks.filter((t) => t.id !== tempId),
      }));
      const message = err instanceof Error ? err.message : "Failed to create task";
      toast.error(message);
      throw err;
    }
  },

  updateTask: async (taskId: string, req: UpdateKanbanTaskRequest) => {
    const prev = _get().tasks;
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, ...req, updated_at: new Date().toISOString() } : t
      ),
    }));

    try {
      await kanbanService.updateTask(taskId, req);
    } catch (err) {
      set({ tasks: prev });
      const message = err instanceof Error ? err.message : "Failed to update task";
      toast.error(message);
    }
  },

  deleteTask: async (taskId: string) => {
    const prev = _get().tasks;
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== taskId),
    }));

    try {
      await kanbanService.deleteTask(taskId);
    } catch (err) {
      set({ tasks: prev });
      const message = err instanceof Error ? err.message : "Failed to delete task";
      toast.error(message);
    }
  },

  moveTask: async (taskId: string, newStatus: KanbanTaskStatus) => {
    await _get().updateTask(taskId, { status: newStatus });
  },

  reorderTasks: async (taskIds: string[]) => {
    try {
      await kanbanService.reorderTasks(taskIds);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to reorder tasks";
      toast.error(message);
    }
  },
}));
