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
import { CharacterNode } from "./CharacterNode";
import { getLayoutedElements, NODE_COLORS } from "./layout";
import { useI18n } from "@/lib/i18n";
import type { Character, CharacterRelationship } from "@/types";

const nodeTypes = { character: CharacterNode };

interface CharacterGraphProps {
  characters: Character[];
  relationships: CharacterRelationship[];
  onNodeClick?: (character: Character) => void;
}

export function CharacterGraph({ characters, relationships, onNodeClick }: CharacterGraphProps) {
  const { t } = useI18n();
  const [filterType, setFilterType] = useState<string>("all");

  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    const nodes: Node[] = characters.map((c) => ({
      id: c.id,
      type: "character",
      position: { x: 0, y: 0 },
      data: {
        name: c.name,
        role: c.role,
        traits: c.traits,
        description: c.description,
      },
    }));

    const edges: Edge[] = relationships.map((r) => ({
      id: r.id,
      source: r.character_a_id,
      target: r.character_b_id,
      label: r.relationship_type,
      type: "default",
      animated: false,
      style: { stroke: "#94a3b8", strokeWidth: 1.5 },
      labelStyle: { fontSize: 10, fill: "#64748b" },
      labelBgStyle: { fill: "white", fillOpacity: 0.8 },
      labelBgPadding: [4, 2] as [number, number],
    }));

    return getLayoutedElements(nodes, edges, { direction: "LR", ranksep: 100, nodesep: 60 });
  }, [characters, relationships]);

  const [nodes, , onNodesChange] = useNodesState(layoutedNodes);
  const [edges, , onEdgesChange] = useEdgesState(layoutedEdges);

  const filteredNodes = useMemo(() => {
    if (filterType === "all") return nodes;
    return nodes.filter((n) => {
      const role = (n.data as { role?: string })?.role?.toLowerCase() || "";
      return role.includes(filterType);
    });
  }, [nodes, filterType]);

  const filteredEdges = useMemo(() => {
    if (filterType === "all") return edges;
    const nodeIds = new Set(filteredNodes.map((n) => n.id));
    return edges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target));
  }, [edges, filterType, filteredNodes]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onNodeClick) {
        const char = characters.find((c) => c.id === node.id);
        if (char) onNodeClick(char);
      }
    },
    [characters, onNodeClick],
  );

  const roleTypes = useMemo(() => {
    const types = new Set<string>();
    characters.forEach((c) => {
      if (c.role) types.add(c.role.toLowerCase());
    });
    return Array.from(types);
  }, [characters]);

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
        {roleTypes.map((type) => (
          <button
            key={type}
            onClick={() => setFilterType(type)}
            className={`text-xs px-2 py-1 rounded-md transition-colors ${
              filterType === type ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            {type}
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
            nodeColor={(n) => {
              const role = (n.data as { role?: string })?.role?.toLowerCase() || "";
              return Object.entries(NODE_COLORS).find(([k]) => role.includes(k))?.[1] || "#6b7280";
            }}
            maskColor="rgba(0,0,0,0.1)"
          />
        </ReactFlow>
      </div>
    </div>
  );
}
