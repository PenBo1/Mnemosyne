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
import { WorldNode } from "./WorldNode";
import { getLayoutedElements, CATEGORY_COLORS } from "./layout";
import { useI18n } from "@/lib/i18n";
import type { WorldSetting } from "@/types";

const nodeTypes = { world: WorldNode };

interface WorldNetworkProps {
  items: WorldSetting[];
  onNodeClick?: (item: WorldSetting) => void;
}

function buildEdges(items: WorldSetting[]): Edge[] {
  const edges: Edge[] = [];
  const tagIndex = new Map<string, string[]>();

  for (const item of items) {
    for (const tag of item.tags) {
      const lower = tag.toLowerCase();
      if (!tagIndex.has(lower)) tagIndex.set(lower, []);
      tagIndex.get(lower)!.push(item.id);
    }
  }

  const seen = new Set<string>();
  for (const ids of tagIndex.values()) {
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const key = [ids[i], ids[j]].sort().join("-");
        if (seen.has(key)) continue;
        seen.add(key);
        edges.push({
          id: key,
          source: ids[i],
          target: ids[j],
          type: "default",
          style: { stroke: "#cbd5e1", strokeWidth: 1, strokeDasharray: "4 4" },
        });
      }
    }
  }

  return edges;
}

export function WorldNetwork({ items, onNodeClick }: WorldNetworkProps) {
  const { t } = useI18n();
  const [filterCategory, setFilterCategory] = useState<string>("all");

  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    const nodes: Node[] = items.map((item) => ({
      id: item.id,
      type: "world",
      position: { x: 0, y: 0 },
      data: {
        name: item.name,
        category: item.category,
        description: item.description,
        tags: item.tags,
      },
    }));

    const edges = buildEdges(items);
    return getLayoutedElements(nodes, edges, { direction: "TB", ranksep: 80, nodesep: 60 });
  }, [items]);

  const [nodes, , onNodesChange] = useNodesState(layoutedNodes);
  const [edges, , onEdgesChange] = useEdgesState(layoutedEdges);

  const filteredNodes = useMemo(() => {
    if (filterCategory === "all") return nodes;
    return nodes.filter((n) => (n.data as { category?: string })?.category === filterCategory);
  }, [nodes, filterCategory]);

  const filteredEdges = useMemo(() => {
    if (filterCategory === "all") return edges;
    const nodeIds = new Set(filteredNodes.map((n) => n.id));
    return edges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target));
  }, [edges, filterCategory, filteredNodes]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onNodeClick) {
        const item = items.find((i) => i.id === node.id);
        if (item) onNodeClick(item);
      }
    },
    [items, onNodeClick],
  );

  const categories = useMemo(() => {
    const cats = new Set<string>();
    items.forEach((i) => cats.add(i.category));
    return Array.from(cats);
  }, [items]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 mb-3 flex-wrap">
        <button
          onClick={() => setFilterCategory("all")}
          className={`text-xs px-2 py-1 rounded-md transition-colors ${
            filterCategory === "all" ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-muted/80"
          }`}
        >
          {t.viz.all}
        </button>
        {categories.map((cat) => (
          <button
            key={cat}
            onClick={() => setFilterCategory(cat)}
            className={`text-xs px-2 py-1 rounded-md transition-colors flex items-center gap-1 ${
              filterCategory === cat ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            <div className="size-1.5 rounded-full" style={{ backgroundColor: CATEGORY_COLORS[cat] || "#6b7280" }} />
            {cat}
          </button>
        ))}
      </div>
      <div className="flex-1 rounded-lg border bg-background overflow-hidden">
        <ReactFlow
          nodes={filteredNodes}
          edges={filteredEdges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={handleNodeClick}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.3 }}
          minZoom={0.3}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
          <Controls />
          <MiniMap
            nodeColor={(n) => CATEGORY_COLORS[(n.data as { category?: string })?.category || ""] || "#6b7280"}
            maskColor="rgba(0,0,0,0.1)"
          />
        </ReactFlow>
      </div>
    </div>
  );
}
