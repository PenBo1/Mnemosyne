import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { CATEGORY_COLORS } from "./layout";
import { useI18n } from "@/lib/i18n";

interface WorldNodeData {
  name: string;
  category: string;
  description: string;
  tags: string[];
  [key: string]: unknown;
}

export const WorldNode = memo(function WorldNode({ data }: NodeProps) {
  const { t } = useI18n();
  const d = data as WorldNodeData;
  const color = CATEGORY_COLORS[d.category] || "#6b7280";
  const categoryLabel = t.viz.world[d.category as keyof typeof t.viz.world] || d.category;

  return (
    <div className="relative bg-card border rounded-xl shadow-sm min-w-[140px] max-w-[200px]">
      <Handle type="target" position={Position.Top} className="!bg-transparent !border-0 !w-0 !h-0" />
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 mb-1">
          <div className="size-2 rounded-full shrink-0" style={{ backgroundColor: color }} />
          <span className="font-medium text-sm truncate flex-1">{d.name}</span>
        </div>
        <span
          className="text-[10px] px-1.5 py-0.5 rounded-full inline-block"
          style={{ backgroundColor: `${color}20`, color }}
        >
          {categoryLabel}
        </span>
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
      <Handle type="source" position={Position.Bottom} className="!bg-transparent !border-0 !w-0 !h-0" />
    </div>
  );
});
