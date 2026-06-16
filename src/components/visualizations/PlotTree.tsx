import { useMemo, useCallback } from "react";
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
import { PlotNode } from "./PlotNode";
import { getLayoutedElements } from "./layout";
import type { PlotPoint } from "@/types";

const nodeTypes = { plot: PlotNode };

interface PlotTreeProps {
  points: PlotPoint[];
  onNodeClick?: (point: PlotPoint) => void;
}

export function PlotTree({ points, onNodeClick }: PlotTreeProps) {
  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    const nodes: Node[] = points.map((p) => ({
      id: p.id,
      type: "plot",
      position: { x: 0, y: 0 },
      data: {
        title: p.title,
        type: p.type,
        status: p.status,
        chapterNumber: p.chapter_number,
        description: p.description,
        goals: p.goals,
        conflicts: p.conflicts,
      },
    }));

    const edges: Edge[] = points
      .filter((p) => p.parent_id)
      .map((p) => ({
        id: `${p.parent_id}-${p.id}`,
        source: p.parent_id!,
        target: p.id,
        type: "default",
        animated: p.status === "in_progress",
        style: { stroke: "#94a3b8", strokeWidth: 1.5 },
      }));

    return getLayoutedElements(nodes, edges, { direction: "TB", ranksep: 80, nodesep: 50 });
  }, [points]);

  const [nodes, , onNodesChange] = useNodesState(layoutedNodes);
  const [edges, , onEdgesChange] = useEdgesState(layoutedEdges);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onNodeClick) {
        const point = points.find((p) => p.id === node.id);
        if (point) onNodeClick(point);
      }
    },
    [points, onNodeClick],
  );

  return (
    <div className="flex-1 rounded-lg border bg-background overflow-hidden">
      <ReactFlow
        nodes={nodes}
        edges={edges}
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
          nodeColor={() => "#94a3b8"}
          maskColor="rgba(0,0,0,0.1)"
        />
      </ReactFlow>
    </div>
  );
}
