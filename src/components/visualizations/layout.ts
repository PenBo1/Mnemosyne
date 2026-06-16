import Dagre from "@dagrejs/dagre";
import type { Node, Edge } from "@xyflow/react";

interface LayoutOptions {
  direction?: "TB" | "LR" | "BT" | "RL";
  nodeWidth?: number;
  nodeHeight?: number;
  ranksep?: number;
  nodesep?: number;
}

export function getLayoutedElements(
  nodes: Node[],
  edges: Edge[],
  options: LayoutOptions = {},
): { nodes: Node[]; edges: Edge[] } {
  const {
    direction = "TB",
    nodeWidth = 180,
    nodeHeight = 60,
    ranksep = 60,
    nodesep = 40,
  } = options;

  const g = new Dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: direction, ranksep, nodesep });

  for (const node of nodes) {
    g.setNode(node.id, { width: nodeWidth, height: nodeHeight });
  }
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  Dagre.layout(g);

  const layoutedNodes = nodes.map((node) => {
    const pos = g.node(node.id);
    return {
      ...node,
      position: {
        x: pos.x - nodeWidth / 2,
        y: pos.y - nodeHeight / 2,
      },
    };
  });

  return { nodes: layoutedNodes, edges };
}

export const NODE_COLORS: Record<string, string> = {
  protagonist: "#f59e0b",
  antagonist: "#ef4444",
  ally: "#10b981",
  minor: "#3b82f6",
  mentioned: "#6b7280",
  主角: "#f59e0b",
  反派: "#ef4444",
  盟友: "#10b981",
  配角: "#3b82f6",
  提及: "#6b7280",
};

export const CATEGORY_COLORS: Record<string, string> = {
  location: "#3b82f6",
  faction: "#ef4444",
  species: "#10b981",
  culture: "#8b5cf6",
  history: "#f59e0b",
  magic_system: "#ec4899",
  language: "#06b6d4",
  architecture: "#f97316",
};

export const EVENT_TYPE_COLORS: Record<string, string> = {
  event: "#3b82f6",
  milestone: "#f59e0b",
  turning_point: "#ef4444",
};

export const HOOK_STATUS_COLORS: Record<string, string> = {
  Open: "#f59e0b",
  Progressing: "#3b82f6",
  Deferred: "#6b7280",
  Resolved: "#10b981",
};
