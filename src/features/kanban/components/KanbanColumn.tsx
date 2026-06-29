import type { KanbanTask, KanbanColumn } from "@/shared/types";
import { KanbanCard } from "./KanbanCard";
import { cn } from "@/shared/utils";

interface KanbanColumnProps {
  column: KanbanColumn;
  tasks: KanbanTask[];
  onDragStart: (e: React.DragEvent, taskId: string) => void;
  onDrop: (e: React.DragEvent, statusKey: string) => void;
  onDragOver: (e: React.DragEvent) => void;
  onEditTask: (taskId: string) => void;
  onDeleteTask: (taskId: string) => Promise<void>;
}

export function KanbanColumnComponent({
  column,
  tasks,
  onDragStart,
  onDrop,
  onDragOver,
  onEditTask,
  onDeleteTask,
}: KanbanColumnProps) {
  const isWipExceeded =
    column.wip_limit !== null && tasks.length > column.wip_limit;

  return (
    <div
      className={cn(
        "flex flex-col min-w-[260px] w-[260px] bg-muted/50 rounded-lg border",
        isWipExceeded && "border-destructive"
      )}
      onDragOver={onDragOver}
      onDrop={(e) => onDrop(e, column.status_key)}
    >
      <div className="flex items-center gap-2 px-3 py-2 border-b">
        <div
          className="w-2.5 h-2.5 rounded-full"
          style={{ backgroundColor: column.color }}
        />
        <span className="text-sm font-medium">{column.name}</span>
        <span className="text-xs text-muted-foreground ml-auto">
          {tasks.length}
          {column.wip_limit !== null && (
            <span className={cn(isWipExceeded && "text-destructive")}>
              /{column.wip_limit}
            </span>
          )}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-2 min-h-[100px]">
        {tasks.length === 0 ? (
          <div className="flex items-center justify-center h-20 text-xs text-muted-foreground border border-dashed rounded-md">
            Drop tasks here
          </div>
        ) : (
          tasks.map((task) => (
            <KanbanCard
              key={task.id}
              task={task}
              onDragStart={onDragStart}
              onEdit={() => onEditTask(task.id)}
              onDelete={() => onDeleteTask(task.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}
