import { useCallback, useEffect } from "react";
import { useKanbanStore } from "@/stores/kanban";
import type { CreateKanbanTaskRequest, KanbanTaskStatus } from "@/types";

export function useKanban(novelId: string | null) {
  const {
    tasks,
    columns,
    loading,
    error,
    loadTasks,
    loadColumns,
    createTask,
    updateTask,
    deleteTask,
    moveTask,
    reorderTasks,
  } = useKanbanStore();

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
