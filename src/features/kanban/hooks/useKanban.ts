import { useCallback, useEffect } from "react";
import { useKanbanStore } from "@/stores/kanban";
import type { CreateKanbanTaskRequest, KanbanTaskStatus } from "@/shared/types";

export function useKanban(novelId: string | null) {
  const tasks = useKanbanStore((s) => s.tasks);
  const columns = useKanbanStore((s) => s.columns);
  const loading = useKanbanStore((s) => s.loading);
  const error = useKanbanStore((s) => s.error);
  const loadTasks = useKanbanStore((s) => s.loadTasks);
  const loadColumns = useKanbanStore((s) => s.loadColumns);
  const createTask = useKanbanStore((s) => s.createTask);
  const updateTask = useKanbanStore((s) => s.updateTask);
  const deleteTask = useKanbanStore((s) => s.deleteTask);
  const moveTask = useKanbanStore((s) => s.moveTask);
  const reorderTasks = useKanbanStore((s) => s.reorderTasks);

  useEffect(() => {
    if (novelId) {
      loadTasks(novelId);
      loadColumns(novelId);
    }
  }, [novelId, loadTasks, loadColumns]);

  const handleCreate = useCallback(
    async (req: CreateKanbanTaskRequest) => {
      if (!novelId) return;
      return createTask(novelId, req);
    },
    [novelId, createTask]
  );

  const handleMove = useCallback(
    async (taskId: string, newStatus: KanbanTaskStatus) => {
      await moveTask(taskId, newStatus);
    },
    [moveTask]
  );

  const tasksByStatus = useCallback(
    (status: string) => tasks.filter((t) => t.status === status),
    [tasks]
  );

  return {
    tasks,
    columns,
    loading,
    error,
    createTask: handleCreate,
    updateTask,
    deleteTask,
    moveTask: handleMove,
    reorderTasks,
    tasksByStatus,
    reload: () => {
      if (novelId) {
        loadTasks(novelId);
        loadColumns(novelId);
      }
    },
  };
}
