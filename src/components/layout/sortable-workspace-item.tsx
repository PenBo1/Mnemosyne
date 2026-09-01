/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SortableWorkspaceItem - 可拖拽排序的工作区条目组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { memo } from "react";
import { WorkspaceItem } from "./workspace-item";
import type { Workspace } from "@/features/workspace/types";
import type { AppPage } from "@/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface SortableWorkspaceItemProps {
  ws: Workspace;
  isActive: boolean;
  isExpanded: boolean;
  viewMode: "sessions" | "files";
  currentPage: AppPage;
  onToggle: (open: boolean, wsId: string, viewMode: "sessions" | "files") => void;
  onSwitchView: (wsId: string) => void;
  onRemove: (wsId: string) => void;
  onNavigate: (page: AppPage) => void;
}

// ── 可排序工作区条目组件 ────────────────────────────────────────────────────

export const SortableWorkspaceItem = memo(function SortableWorkspaceItem({
  ws,
  isActive,
  isExpanded,
  viewMode,
  currentPage,
  onToggle,
  onSwitchView,
  onRemove,
  onNavigate,
}: SortableWorkspaceItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: ws.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <WorkspaceItem
        ws={ws}
        isActive={isActive}
        isExpanded={isExpanded}
        viewMode={viewMode}
        currentPage={currentPage}
        onToggle={onToggle}
        onSwitchView={onSwitchView}
        onRemove={onRemove}
        onNavigate={onNavigate}
      />
    </div>
  );
});