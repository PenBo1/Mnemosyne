import type { KanbanTask } from "@/types";
import { cn } from "@/lib/utils";
import { GripVertical, Trash2, Edit2 } from "lucide-react";

interface KanbanCardProps {
  task: KanbanTask;
  onDragStart: (e: React.DragEvent, taskId: string) => void;
  onEdit: () => void;
  onDelete: () => void;
}

const PRIORITY_COLORS: Record<string, string> = {
  urgent: "bg-red-500",
  high: "bg-orange-500",
  medium: "bg-yellow-500",
  low: "bg-green-500",
};

export function KanbanCard({ task, onDragStart, onEdit, onDelete }: KanbanCardProps) {
  return (
    <div
      className="group bg-background border rounded-md p-2.5 shadow-sm cursor-grab active:cursor-grabbing hover:shadow-md transition-shadow"
      draggable
      onDragStart={(e) => onDragStart(e, task.id)}
    >
      <div className="flex items-start gap-2">
        <GripVertical className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity mt-0.5 shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5 mb-1">
            <div
              className={cn("w-1.5 h-1.5 rounded-full shrink-0", PRIORITY_COLORS[task.priority] ?? "bg-gray-400")}
            />
            <span className="text-sm font-medium truncate">{task.title}</span>
          </div>

          {task.description && (
            <p className="text-xs text-muted-foreground line-clamp-2 mb-1.5">
              {task.description}
            </p>
          )}

          <div className="flex items-center gap-1.5 flex-wrap">
            {task.assigned_agent && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-primary/10 text-primary">
                {task.assigned_agent}
              </span>
            )}
            {task.chapter_id && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-secondary text-secondary-foreground">
                Ch.
              </span>
            )}
            {task.tags.map((tag) => (
              <span
                key={tag}
                className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
              >
                {tag}
              </span>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
          <button
            className="p-1 rounded hover:bg-muted"
            onClick={(e) => {
              e.stopPropagation();
              onEdit();
            }}
          >
            <Edit2 className="h-3 w-3" />
          </button>
          <button
            className="p-1 rounded hover:bg-destructive/10 text-destructive"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
          >
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>
    </div>
  );
}
