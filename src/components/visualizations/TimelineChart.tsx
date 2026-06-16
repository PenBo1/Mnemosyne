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
import { TimelineNode } from "./TimelineNode";
import { EVENT_TYPE_COLORS } from "./layout";
import { useI18n } from "@/lib/i18n";
import type { TimelineEvent } from "@/types";

const nodeTypes = { timeline: TimelineNode };

interface TimelineChartProps {
  events: TimelineEvent[];
  onNodeClick?: (event: TimelineEvent) => void;
}

export function TimelineChart({ events, onNodeClick }: TimelineChartProps) {
  const { t } = useI18n();
  const [filterType, setFilterType] = useState<string>("all");

  const sortedEvents = useMemo(
    () => [...events].sort((a, b) => a.sort_order - b.sort_order),
    [events],
  );

  const { initialNodes, initialEdges } = useMemo(() => {
    const nodes: Node[] = sortedEvents.map((ev) => ({
      id: ev.id,
      type: "timeline",
      position: { x: 0, y: 0 },
      data: {
        title: ev.title,
        description: ev.description,
        eventDate: ev.event_date,
        eventType: ev.event_type,
        chapterNumber: ev.chapter_number,
        tags: ev.tags,
      },
    }));

    const edges: Edge[] = sortedEvents.slice(1).map((ev, i) => ({
      id: `edge-${sortedEvents[i].id}-${ev.id}`,
      source: sortedEvents[i].id,
      target: ev.id,
      type: "default",
      animated: ev.event_type === "turning_point",
      style: { stroke: "#94a3b8", strokeWidth: 1.5 },
    }));

    return { initialNodes: nodes, initialEdges: edges };
  }, [sortedEvents]);

  const [nodes, , onNodesChange] = useNodesState(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  const filteredNodes = useMemo(() => {
    if (filterType === "all") return nodes;
    return nodes.filter((n) => (n.data as { eventType?: string })?.eventType === filterType);
  }, [nodes, filterType]);

  const filteredEdges = useMemo(() => {
    if (filterType === "all") return edges;
    const nodeIds = new Set(filteredNodes.map((n) => n.id));
    return edges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target));
  }, [edges, filterType, filteredNodes]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onNodeClick) {
        const event = sortedEvents.find((e) => e.id === node.id);
        if (event) onNodeClick(event);
      }
    },
    [sortedEvents, onNodeClick],
  );

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 mb-3 flex-wrap">
        <button
          onClick={() => setFilterType("all")}
          className={`text-xs px-2 py-1 rounded-md transition-colors ${
            filterType === "all" ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-muted/80"
          }`}
        >
          {t.viz.all}
        </button>
        {Object.entries(EVENT_TYPE_COLORS).map(([type, color]) => (
          <button
            key={type}
            onClick={() => setFilterType(type)}
            className={`text-xs px-2 py-1 rounded-md transition-colors flex items-center gap-1 ${
              filterType === type ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            <div className="size-1.5 rounded-full" style={{ backgroundColor: color }} />
            {t.viz.timeline[type as keyof typeof t.viz.timeline]}
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
            nodeColor={(n) => EVENT_TYPE_COLORS[(n.data as { eventType?: string })?.eventType || "event"] || "#3b82f6"}
            maskColor="rgba(0,0,0,0.1)"
          />
        </ReactFlow>
      </div>
    </div>
  );
}
