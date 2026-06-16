import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { NODE_COLORS } from "./layout";

interface CharacterNodeData {
  name: string;
  role: string;
  traits: string[];
  description: string;
  [key: string]: unknown;
}

export const CharacterNode = memo(function CharacterNode({ data }: NodeProps) {
  const d = data as CharacterNodeData;
  const roleKey = d.role?.toLowerCase() || "";
  const color = Object.entries(NODE_COLORS).find(([k]) =>
    roleKey.includes(k),
  )?.[1] || "#6b7280";

  return (
    <div
      className="relative bg-card border rounded-xl shadow-sm min-w-[140px] max-w-[200px]"
      style={{ borderColor: color }}
    >
      <Handle type="target" position={Position.Top} className="!bg-transparent !border-0 !w-0 !h-0" />
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 mb-1">
          <div className="size-2 rounded-full shrink-0" style={{ backgroundColor: color }} />
          <span className="font-medium text-sm truncate">{d.name}</span>
        </div>
        {d.role && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded-full inline-block"
            style={{ backgroundColor: `${color}20`, color }}
          >
            {d.role}
          </span>
        )}
        {d.traits && d.traits.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1.5">
            {d.traits.slice(0, 3).map((trait: string) => (
              <span key={trait} className="text-[9px] bg-muted px-1 py-0.5 rounded">
                {trait}
              </span>
            ))}
          </div>
        )}
      </div>
      <Handle type="source" position={Position.Bottom} className="!bg-transparent !border-0 !w-0 !h-0" />
    </div>
  );
});
