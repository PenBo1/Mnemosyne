import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { EVENT_TYPE_COLORS } from "./layout";
import { useI18n } from "@/lib/i18n";

interface TimelineNodeData {
  title: string;
  description: string;
  eventDate: string;
  eventType: string;
  chapterNumber: number | null;
  tags: string[];
  [key: string]: unknown;
}

export const TimelineNode = memo(function TimelineNode({ data }: NodeProps) {
  const { t } = useI18n();
  const d = data as TimelineNodeData;
  const color = EVENT_TYPE_COLORS[d.eventType] || "#3b82f6";
  const typeLabel = t.viz.timeline[d.eventType as keyof typeof t.viz.timeline] || d.eventType;

  return (
    <div className="relative bg-card border rounded-xl shadow-sm min-w-[150px] max-w-[200px]">
      <Handle type="target" position={Position.Left} className="!bg-transparent !border-0 !w-0 !h-0" />
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 mb-1">
          <div className="size-2 rounded-full shrink-0" style={{ backgroundColor: color }} />
          <span className="font-medium text-sm truncate flex-1">{d.title}</span>
        </div>
        <div className="flex items-center gap-2">
          <span
            className="text-[10px] px-1.5 py-0.5 rounded-full"
            style={{ backgroundColor: `${color}20`, color }}
          >
            {typeLabel}
          </span>
          {d.eventDate && (
            <span className="text-[10px] text-muted-foreground">{d.eventDate}</span>
          )}
          {d.chapterNumber != null && (
            <span className="text-[10px] text-muted-foreground">Ch.{d.chapterNumber}</span>
          )}
        </div>
        {d.description && (
          <p className="text-[11px] text-muted-foreground mt-1.5 line-clamp-2">{d.description}</p>
        )}
        {d.tags && d.tags.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1.5">
            {d.tags.slice(0, 3).map((tag: string) => (
              <span key={tag} className="text-[9px] bg-muted px-1 py-0.5 rounded">
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>
      <Handle type="source" position={Position.Right} className="!bg-transparent !border-0 !w-0 !h-0" />
    </div>
  );
});
