// ── Kanban ─────────────────────────────────────────────────
//
// KanbanState（含 actions 的 store 状态接口）也在此处定义。

export type KanbanTaskStatus = "plan" | "compose" | "write" | "audit" | "revise" | "done" | "cancelled";
export type KanbanPriority = "low" | "medium" | "high" | "urgent";

export interface KanbanTask {
  id: string;
  novel_id: string;
  title: string;
  description: string;
  status: KanbanTaskStatus;
  priority: KanbanPriority;
  assigned_agent: string | null;
  chapter_id: string | null;
  parent_task_id: string | null;
  tags: string[];
  sort_order: number;
  due_date: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateKanbanTaskRequest {
  title: string;
  description?: string;
  status?: KanbanTaskStatus;
  priority?: KanbanPriority;
  assigned_agent?: string;
  chapter_id?: string;
  parent_task_id?: string;
  tags?: string[];
  due_date?: string;
}

export interface UpdateKanbanTaskRequest {
  title?: string;
  description?: string;
  status?: KanbanTaskStatus;
  priority?: KanbanPriority;
  assigned_agent?: string;
  chapter_id?: string;
  parent_task_id?: string;
  sort_order?: number;
  due_date?: string;
  tags?: string[];
}

export interface KanbanColumn {
  id: string;
  novel_id: string;
  name: string;
  status_key: string;
  color: string;
  sort_order: number;
  wip_limit: number | null;
  created_at: string;
}

export interface CreateKanbanColumnRequest {
  name: string;
  status_key: string;
  color?: string;
  sort_order?: number;
  wip_limit?: number;
}

export interface UpdateKanbanColumnRequest {
  name?: string;
  color?: string;
  sort_order?: number;
  wip_limit?: number;
}

export interface KanbanState {
  tasks: KanbanTask[];
  columns: KanbanColumn[];
  loading: boolean;
  error: string | null;
  loadTasks: (novelId: string) => Promise<void>;
  loadColumns: (novelId: string) => Promise<void>;
  createTask: (novelId: string, req: CreateKanbanTaskRequest) => Promise<KanbanTask>;
  updateTask: (taskId: string, req: UpdateKanbanTaskRequest) => Promise<void>;
  deleteTask: (taskId: string) => Promise<void>;
  moveTask: (taskId: string, newStatus: KanbanTaskStatus) => Promise<void>;
  reorderTasks: (taskIds: string[]) => Promise<void>;
}
