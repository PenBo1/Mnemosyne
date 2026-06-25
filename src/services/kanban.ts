import { ipc } from "@/lib/ipc";
import type {
  KanbanTask,
  KanbanColumn,
  CreateKanbanTaskRequest,
  UpdateKanbanTaskRequest,
  CreateKanbanColumnRequest,
  UpdateKanbanColumnRequest,
} from "@/types";

export async function createTask(
  novelId: string,
  req: CreateKanbanTaskRequest
): Promise<KanbanTask> {
  return ipc<KanbanTask>("kanban_create_task", {
    novelId,
    title: req.title,
    description: req.description,
    status: req.status,
    priority: req.priority,
    assignedAgent: req.assigned_agent,
    chapterId: req.chapter_id,
    parentTaskId: req.parent_task_id,
    tags: req.tags,
    dueDate: req.due_date,
  });
}

export async function getTasks(
  novelId: string,
  statusFilter?: string
): Promise<KanbanTask[]> {
  return ipc<KanbanTask[]>("kanban_get_tasks", {
    novelId,
    statusFilter,
  });
}

export async function updateTask(
  taskId: string,
  req: UpdateKanbanTaskRequest
): Promise<KanbanTask> {
  return ipc<KanbanTask>("kanban_update_task", {
    taskId,
    title: req.title,
    description: req.description,
    status: req.status,
    priority: req.priority,
    assignedAgent: req.assigned_agent,
    chapterId: req.chapter_id,
    parentTaskId: req.parent_task_id,
    sortOrder: req.sort_order,
    dueDate: req.due_date,
    tags: req.tags,
  });
}

export async function deleteTask(taskId: string): Promise<void> {
  return ipcVoid("kanban_delete_task", { taskId });
}

export async function reorderTasks(taskIds: string[]): Promise<void> {
  return ipcVoid("kanban_reorder_tasks", { taskIds });
}

export async function getColumns(novelId: string): Promise<KanbanColumn[]> {
  return ipc<KanbanColumn[]>("kanban_get_columns", { novelId });
}

export async function createColumn(
  novelId: string,
  req: CreateKanbanColumnRequest
): Promise<KanbanColumn> {
  return ipc<KanbanColumn>("kanban_create_column", {
    novelId,
    name: req.name,
    statusKey: req.status_key,
    color: req.color,
    sortOrder: req.sort_order,
    wipLimit: req.wip_limit,
  });
}

export async function updateColumn(
  columnId: string,
  req: UpdateKanbanColumnRequest
): Promise<KanbanColumn> {
  return ipc<KanbanColumn>("kanban_update_column", {
    columnId,
    name: req.name,
    color: req.color,
    sortOrder: req.sort_order,
    wipLimit: req.wip_limit,
  });
}

export async function deleteColumn(columnId: string): Promise<void> {
  return ipcVoid("kanban_delete_column", { columnId });
}

async function ipcVoid(command: string, args?: Record<string, unknown>): Promise<void> {
  await ipc<unknown>(command, args);
}
