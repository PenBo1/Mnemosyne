import { useState, useCallback } from "react";
import { useI18n } from "@/shared/i18n";
import { useKanban } from "@/features/kanban/hooks/useKanban";
import { useWorkspaceStore } from "@/stores/workspace";
import { KanbanBoard } from "@/features/kanban/components/KanbanBoard";
import { KanbanTaskDialog } from "@/features/kanban/components/KanbanTaskDialog";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import type { CreateKanbanTaskRequest, UpdateKanbanTaskRequest } from "@/shared/types";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";

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
      <PageContainer>
        <EmptyState
          title={t.kanban?.empty?.board ?? "Select a novel first"}
        />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.kanban?.title ?? "Kanban Board"}</PageTitle>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDialogOpen(true)}
          >
            <Plus data-icon="inline-start" />
            {t.kanban?.newTask ?? "New Task"}
          </Button>
        </PageActions>
      </PageHeader>

      <div className="flex-1 min-h-0 overflow-hidden">
        {loading ? (
          <LoadingState />
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
    </PageContainer>
  );
}
