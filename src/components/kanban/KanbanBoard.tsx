import { useMemo } from "react";
import type { KanbanTask, KanbanColumn, KanbanTaskStatus } from "@/types";
import { KanbanColumnComponent } from "./KanbanColumn";

interface KanbanBoardProps {
  tasks: KanbanTask[];
  columns: KanbanColumn[];
  onMoveTask: (taskId: string, newStatus: KanbanTaskStatus) => Promise<void>;
  onEditTask: (taskId: string) => void;
  onDeleteTask: (taskId: string) => Promise<void>;
}

const DEFAULT_COLUMNS: { status_key: string; name: string; color: string }[] = [
  { status_key: "plan", name: "Plan", color: "#6366f1" },
  { status_key: "compose", name: "Compose", color: "#8b5cf6" },
  { status_key: "write", name: "Write", color: "#ec4899" },
  { status_key: "audit", name: "Audit", color: "#f59e0b" },
  { status_key: "revise", name: "Revise", color: "#ef4444" },
  { status_key: "done", name: "Done", color: "#22c55e" },
];

export function KanbanBoard({
  tasks,
  columns,
  onMoveTask,
  onEditTask,
  onDeleteTask,
}: KanbanBoardProps) {
  const displayColumns = useMemo(() => {
    if (columns.length > 0) return columns;
    return DEFAULT_COLUMNS.map((c, i) => ({
      id: c.status_key,
      novel_id: "",
      name: c.name,
      status_key: c.status_key,
      color: c.color,
      sort_order: i,
      wip_limit: null,
      created_at: "",
    }));
  }, [columns]);

  const tasksByStatus = useMemo(() => {
    const map: Record<string, KanbanTask[]> = {};
    for (const col of displayColumns) {
      map[col.status_key] = [];
    }
    for (const task of tasks) {
      if (map[task.status]) {
        map[task.status].push(task);
      }
    }
    return map;
  }, [tasks, displayColumns]);

  const handleDragStart = (e: React.DragEvent, taskId: string) => {
    e.dataTransfer.setData("text/plain", taskId);
    e.dataTransfer.effectAllowed = "move";
  };

  const handleDrop = (e: React.DragEvent, statusKey: string) => {
    e.preventDefault();
    const taskId = e.dataTransfer.getData("text/plain");
    if (taskId) {
      onMoveTask(taskId, statusKey as KanbanTaskStatus);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  };

  return (
    <div className="flex gap-3 h-full overflow-x-auto p-4">
      {displayColumns.map((col) => (
        <KanbanColumnComponent
          key={col.id}
          column={col}
          tasks={tasksByStatus[col.status_key] ?? []}
          onDragStart={handleDragStart}
          onDrop={handleDrop}
          onDragOver={handleDragOver}
          onEditTask={onEditTask}
          onDeleteTask={onDeleteTask}
        />
      ))}
    </div>
  );
}
