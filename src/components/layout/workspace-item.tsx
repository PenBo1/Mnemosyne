/**
 * ═══════════════════════════════════════════════════════════════════════════
 * WorkspaceItem - 侧边栏工作区条目组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo, lazy, Suspense } from "react";
import {
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuSub,
  SidebarMenuSubItem,
  SidebarMenuSubButton,
} from "@/components/ui/sidebar";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import {
  FolderIcon,
  ChevronRightIcon,
  FilesIcon,
  MessageSquareIcon,
  MoreHorizontalIcon,
  CpuIcon,
  WorkflowIcon,
  GitBranchIcon,
  ShieldCheckIcon,
  NetworkIcon,
  BookOpenIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { AppPage } from "@/types";
import type { Workspace } from "@/features/workspace/types/workspace";

// ── 延迟加载组件 ────────────────────────────────────────────────────────────

/** 延迟加载会话列表，避免 layout 静态依赖 @/features/chat */
const SidebarSessionList = lazy(() =>
  import("@/features/chat/components/sidebar-session-list").then((m) => ({
    default: m.SidebarSessionList,
  }))
);

// ── 常量配置 ────────────────────────────────────────────────────────────────

/**
 * 工作区文件视图下的二级菜单项
 * 这些功能依赖工作区上下文（项目目录 / 仓库 / 配置）
 * 从「工具」菜单移到工作区下，通过「会话/文件」切换按钮显示
 */
const WORKSPACE_FILE_ITEMS: { id: "loops" | "pipeline" | "git" | "audit" | "wiki"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "loops", labelKey: "loops", icon: CpuIcon },
  { id: "pipeline", labelKey: "pipeline", icon: WorkflowIcon },
  { id: "git", labelKey: "git", icon: GitBranchIcon },
  { id: "audit", labelKey: "audit", icon: ShieldCheckIcon },
  { id: "wiki", labelKey: "wiki", icon: NetworkIcon },
];

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface WorkspaceItemProps {
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

// ── 工作区条目组件 ──────────────────────────────────────────────────────────

/**
 * 侧边栏工作区条目
 * 含会话/文件视图切换与二级文件菜单
 */
export const WorkspaceItem = memo(function WorkspaceItem({
  ws,
  isActive,
  isExpanded,
  viewMode,
  currentPage,
  onToggle,
  onSwitchView,
  onRemove,
  onNavigate,
}: WorkspaceItemProps) {
  const { t } = useI18n();
  return (
    <Collapsible
      open={isExpanded}
      onOpenChange={(open) => onToggle(open, ws.id, viewMode)}
      className="group/ws"
    >
      <SidebarMenuItem className="group/menu-item">
        <CollapsibleTrigger asChild>
          <SidebarMenuButton isActive={isActive}>
            <FolderIcon className="group-hover/ws:hidden" />
            <ChevronRightIcon
              className={cn(
                "hidden group-hover/ws:block transition-transform duration-200",
                isExpanded && "rotate-90",
              )}
            />
            <span className="flex-1 truncate">{ws.name}</span>
          </SidebarMenuButton>
        </CollapsibleTrigger>
        {/* 工作区悬浮时显示两个按钮：会话/文件切换 + 更多 */}
        <div className="absolute top-1.5 right-1 flex items-center gap-0.5 opacity-0 transition-opacity group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 md:opacity-0">
          <button
            type="button"
            className="flex size-5 items-center justify-center rounded-[calc(var(--radius-sm)-2px)] text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            title={viewMode === "sessions" ? t.sidebar.switchToFiles : t.sidebar.switchToSessions}
            aria-label={viewMode === "sessions" ? t.sidebar.switchToFiles : t.sidebar.switchToSessions}
            onClick={(e) => {
              e.stopPropagation();
              onSwitchView(ws.id);
            }}
          >
            {viewMode === "sessions" ? <FilesIcon className="size-4" /> : <MessageSquareIcon className="size-4" />}
          </button>
          <button
            type="button"
            className="flex size-5 items-center justify-center rounded-[calc(var(--radius-sm)-2px)] text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            title={t.sidebar.more}
            aria-label={t.sidebar.more}
            onClick={(e) => {
              e.stopPropagation();
              onRemove(ws.id);
            }}
          >
            <MoreHorizontalIcon className="size-4" />
          </button>
        </div>
        <CollapsibleContent>
          {viewMode === "sessions" ? (
            <Suspense fallback={null}>
              <SidebarSessionList workspaceId={ws.id} />
            </Suspense>
          ) : (
            <SidebarMenuSub>
              {WORKSPACE_FILE_ITEMS.map((item) => (
                <SidebarMenuSubItem key={item.id}>
                  <SidebarMenuSubButton
                    isActive={currentPage === item.id}
                    onClick={() => onNavigate(item.id)}
                  >
                    <item.icon />
                    <span>{t.sidebar[item.labelKey as keyof typeof t.sidebar] as string}</span>
                  </SidebarMenuSubButton>
                </SidebarMenuSubItem>
              ))}
            </SidebarMenuSub>
          )}
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  );
});
