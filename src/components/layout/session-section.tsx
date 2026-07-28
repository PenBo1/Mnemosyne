/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SessionSection - 侧边栏工作区列表区组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback } from "react";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from "@/components/ui/sidebar";
import { FolderIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useSidebarWorkspaces } from "@/features/workspace/hooks/useSidebarWorkspaces";
import { CreateWorkspaceDialog } from "@/features/workspace/CreateWorkspaceDialog";
import type { AppPage } from "@/types";
import { WorkspaceItem } from "./workspace-item";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface SessionSectionProps {
  currentPage: AppPage;
  onNavigate: (page: AppPage) => void;
}

// ── 工作区列表组件 ──────────────────────────────────────────────────────────

/**
 * 侧边栏工作区列表区
 * 管理工作区增删/展开/视图切换
 * 自治组件：内部调用 useSidebarWorkspaces 并管理工作区展开/视图模式等本地状态
 */
export function SessionSection({ currentPage, onNavigate }: SessionSectionProps) {
  const { t } = useI18n();
  const {
    workspaces,
    activeWorkspaceId,
    setActiveWorkspace,
    removeWorkspace,
    dialogOpen,
    setDialogOpen,
    newWorkspaceName,
    setNewWorkspaceName,
    newWorkspacePath,
    setNewWorkspacePath,
    creating,
    handlePickDirectory,
    handleAddWorkspace,
  } = useSidebarWorkspaces();
  const [expandedWs, setExpandedWs] = useState<string | null>(null);
  /**
   * 每个工作区的视图模式：sessions（会话列表）/ files（文件功能菜单）。
   * 默认 sessions。切换按钮在 hover 时显示在工作区行右侧。
   */
  const [wsViewMode, setWsViewMode] = useState<Record<string, "sessions" | "files">>({});

  const getWsView = useCallback((wsId: string): "sessions" | "files" => {
    return wsViewMode[wsId] ?? "sessions";
  }, [wsViewMode]);

  const toggleWsView = useCallback((wsId: string) => {
    setWsViewMode((prev) => ({
      ...prev,
      [wsId]: prev[wsId] === "files" ? "sessions" : "files",
    }));
  }, []);

  const handleWsToggle = useCallback((open: boolean, wsId: string, viewMode: "sessions" | "files") => {
    setExpandedWs(open ? wsId : null);
    setActiveWorkspace(wsId);
    if (open && viewMode === "sessions") {
      onNavigate("chat");
    }
  }, [setActiveWorkspace, onNavigate]);

  return (
    <SidebarGroup>
      <SidebarGroupLabel className="flex items-center justify-between">
        <span>{t.sidebar.workspaces}</span>
        <CreateWorkspaceDialog
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          name={newWorkspaceName}
          onNameChange={setNewWorkspaceName}
          path={newWorkspacePath}
          onPathChange={setNewWorkspacePath}
          creating={creating}
          onPickDirectory={handlePickDirectory}
          onCreate={handleAddWorkspace}
        />
      </SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu className="gap-1">
          {workspaces.length === 0 ? (
            <SidebarMenuItem>
              <SidebarMenuButton disabled>
                <FolderIcon />
                <span className="text-muted-foreground">{t.sidebar.noWorkspaces}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ) : (
            workspaces.map((ws) => (
              <WorkspaceItem
                key={ws.id}
                ws={ws}
                isActive={activeWorkspaceId === ws.id}
                isExpanded={expandedWs === ws.id}
                viewMode={getWsView(ws.id)}
                currentPage={currentPage}
                onToggle={handleWsToggle}
                onSwitchView={toggleWsView}
                onRemove={removeWorkspace}
                onNavigate={onNavigate}
              />
            ))
          )}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
