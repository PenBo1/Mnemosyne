import { useState, useCallback } from "react";
import { useI18n } from "@/lib/i18n";
import { useKanban } from "@/hooks/useKanban";
import { useWorkspaceStore } from "@/stores/workspace";
import { KanbanBoard } from "@/components/kanban/KanbanBoard";
import { KanbanTaskDialog } from "@/components/kanban/KanbanTaskDialog";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import type { CreateKanbanTaskRequest, UpdateKanbanTaskRequest } from "@/types";

export default function KanbanPage() {
  const { t } = useI18n();
  const activeNovelId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { tasks, columns, loading, createTask, updateTask, deleteTask, moveTask } =
    useKanban(activeNovelId);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingTaskId, setEditingTaskId] = useState<string | null>(null);

  const handleCreate = useCallback(
    async (req: CreateKanbanTaskRequest | UpdateKanbanTaskRequest) => {
      await createTask(req as CreateKanbanTaskRequest);
      setDialogOpen(false);
    },
    [createTask]
  );

  const handleEdit = useCallback(
    async (taskId: string, req: UpdateKanbanTaskRequest) => {
      await updateTask(taskId, req);
      setEditingTaskId(null);
    },
    [updateTask]
  );

  if (!activeNovelId) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        {t.kanban?.empty?.board ?? "Select a novel first"}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <h1 className="text-lg font-semibold">{t.kanban?.title ?? "Kanban Board"}</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDialogOpen(true)}
          >
            <Plus className="h-4 w-4 mr-1" />
            {t.kanban?.newTask ?? "New Task"}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        {loading ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Loading...
          </div>
        ) : (
          <KanbanBoard
            tasks={tasks}
            columns={columns}
            onMoveTask={moveTask}
            onEditTask={(taskId) => setEditingTaskId(taskId)}
            onDeleteTask={deleteTask}
          />
        )}
      </div>

      <KanbanTaskDialog
        open={dialogOpen || editingTaskId !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDialogOpen(false);
            setEditingTaskId(null);
          }
        }}
        task={editingTaskId ? tasks.find((t) => t.id === editingTaskId) ?? null : null}
        onSubmit={
          editingTaskId
            ? (req) => handleEdit(editingTaskId, req)
            : handleCreate
        }
        novelId={activeNovelId}
      />
    </div>
  );
}
