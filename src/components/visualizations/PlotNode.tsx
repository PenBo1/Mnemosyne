import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { useI18n } from "@/lib/i18n";

interface PlotNodeData {
  title: string;
  type: string;
  status: string;
  chapterNumber: number | null;
  description: string;
  goals: string;
  conflicts: string;
  [key: string]: unknown;
}

const STATUS_COLORS: Record<string, string> = {
  planned: "#6b7280",
  in_progress: "#f59e0b",
  completed: "#10b981",
};

const TYPE_ICONS: Record<string, string> = {
  act: "📖",
  chapter: "📄",
  scene: "🎬",
};

export const PlotNode = memo(function PlotNode({ data }: NodeProps) {
  const { t } = useI18n();
  const d = data as PlotNodeData;
  const statusColor = STATUS_COLORS[d.status] || "#6b7280";
  const typeIcon = TYPE_ICONS[d.type] || "📝";
  const statusLabel = t.viz.plot[d.status as keyof typeof t.viz.plot] || d.status;

  return (
    <div className="relative bg-card border rounded-xl shadow-sm min-w-[160px] max-w-[220px]">
      <Handle type="target" position={Position.Top} className="!bg-transparent !border-0 !w-0 !h-0" />
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm">{typeIcon}</span>
          <span className="font-medium text-sm truncate flex-1">{d.title}</span>
        </div>
        <div className="flex items-center gap-2">
          <span
            className="text-[10px] px-1.5 py-0.5 rounded-full"
            style={{ backgroundColor: `${statusColor}20`, color: statusColor }}
          >
            {statusLabel}
          </span>
          {d.chapterNumber != null && (
            <span className="text-[10px] text-muted-foreground">Ch.{d.chapterNumber}</span>
          )}
        </div>
        {d.description && (
          <p className="text-[11px] text-muted-foreground mt-1.5 line-clamp-2">{d.description}</p>
        )}
        {d.goals && (
          <p className="text-[10px] text-muted-foreground/70 mt-1">
            <span className="font-medium">{t.viz.goals}</span> {d.goals.slice(0, 50)}{d.goals.length > 50 ? "..." : ""}
          </p>
        )}
      </div>
      <Handle type="source" position={Position.Bottom} className="!bg-transparent !border-0 !w-0 !h-0" />
    </div>
  );
});
