import { useMemo, useCallback, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  BackgroundVariant,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { cn } from "@/shared/utils";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/shared/i18n";
import type { WikiGraphView, WikiCategory, WikiEntry } from "@/shared/types";

const CATEGORY_COLORS: Record<WikiCategory, string> = {
  general: "#6b7280",
  character: "#3b82f6",
  location: "#10b981",
  event: "#f59e0b",
  concept: "#8b5cf6",
  reference: "#ec4899",
};

interface WikiGraphViewProps {
  graph: WikiGraphView;
  entries: WikiEntry[];
  onNodeClick?: (entry: WikiEntry) => void;
  className?: string;
}

export function WikiGraphViewComponent({ graph, entries, onNodeClick, className }: WikiGraphViewProps) {
  const { t } = useI18n();
  const [filterCategory, setFilterCategory] = useState<WikiCategory | "all">("all");

  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    const nodes: Node[] = graph.nodes.map((n) => ({
      id: n.id,
      type: "default",
      position: { x: 0, y: 0 },
      data: {
        label: n.title,
        category: n.category,
        importance: n.importance,
      },
      style: {
        backgroundColor: CATEGORY_COLORS[n.category] || "#6b7280",
        color: "white",
        borderRadius: "8px",
        padding: "8px 12px",
        fontSize: "12px",
        fontWeight: n.importance >= 5 ? 600 : 400,
        border: n.importance >= 5 ? "2px solid #fff" : "none",
      },
    }));

    const edges: Edge[] = graph.edges.map((l, i) => ({
      id: `link-${i}`,
      source: l.source,
      target: l.target,
      label: l.relation,
      type: "default",
      animated: false,
      style: { stroke: "#94a3b8", strokeWidth: 1.5 },
      labelStyle: { fontSize: 10, fill: "#64748b" },
      labelBgStyle: { fill: "white", fillOpacity: 0.8 },
      labelBgPadding: [4, 2] as [number, number],
    }));

    return { nodes, edges };
  }, [graph]);

  const [nodes, , onNodesChange] = useNodesState(layoutedNodes);
  const [edges, , onEdgesChange] = useEdgesState(layoutedEdges);

  const filteredNodes = useMemo(() => {
    if (filterCategory === "all") return nodes;
    return nodes.filter((n) => {
      const category = (n.data as { category?: WikiCategory })?.category;
      return category === filterCategory;
    });
  }, [nodes, filterCategory]);

  const filteredEdges = useMemo(() => {
    if (filterCategory === "all") return edges;
    const nodeIds = new Set(filteredNodes.map((n) => n.id));
    return edges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target));
  }, [edges, filterCategory, filteredNodes]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onNodeClick) {
        const entry = entries.find((e) => e.id === node.id);
        if (entry) onNodeClick(entry);
      }
    },
    [entries, onNodeClick],
  );

  const categories = useMemo(() => {
    const cats = new Set<WikiCategory>();
    graph.nodes.forEach((n) => cats.add(n.category));
    return Array.from(cats);
  }, [graph.nodes]);

  return (
    <div className={cn("flex flex-col gap-3 h-full", className)}>
      <div className="flex items-center gap-2 flex-wrap">
        <Button
          onClick={() => setFilterCategory("all")}
          variant={filterCategory === "all" ? "default" : "secondary"}
          size="xs"
        >
          {t.viz.all}
        </Button>
        {categories.map((cat) => (
          <Button
            key={cat}
            onClick={() => setFilterCategory(cat)}
            variant={filterCategory === cat ? "default" : "secondary"}
            size="xs"
          >
            {t.wiki.categories[cat]}
          </Button>
        ))}
      </div>
      <div className="flex-1 rounded-[var(--radius-6)] border bg-background overflow-hidden">
        <ReactFlow
          nodes={filteredNodes}
          edges={filteredEdges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={handleNodeClick}
          fitView
          fitViewOptions={{ padding: 0.3 }}
          minZoom={0.3}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
          <Controls />
          <MiniMap
            nodeColor={(n) => {
              const category = (n.data as { category?: WikiCategory })?.category;
              return CATEGORY_COLORS[category || "general"];
            }}
            maskColor="rgba(0,0,0,0.1)"
          />
        </ReactFlow>
      </div>
    </div>
  );
}
