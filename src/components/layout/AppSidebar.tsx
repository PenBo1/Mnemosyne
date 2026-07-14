import { useState } from "react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import {
  SettingsIcon,
  LayersIcon,
  ArrowLeftIcon,
  GlobeIcon,
  ShieldIcon,
  FolderIcon,
  ChevronRightIcon,
  GitBranchIcon,
  MessageSquareIcon,
  TrendingUpIcon,
  BookOpenIcon,
  BookMarkedIcon,
  PuzzleIcon,
  BotIcon,
  WrenchIcon,
  CpuIcon,
  NetworkIcon,
  KeyboardIcon,
  WorkflowIcon,
  InfoIcon,
  BoxesIcon,
  ScrollTextIcon,
  HardDriveIcon,
  RadarIcon,
  SparklesIcon,
  HeartIcon,
  BrainIcon,
  ScrollIcon,
  CalendarClockIcon,
  FolderCogIcon,
  GaugeIcon,
  ShieldCheckIcon,
  UserIcon,
  FilesIcon,
  MoreHorizontalIcon,
  PaletteIcon,
} from "lucide-react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { useAgentStore } from "@/features/chat/store";
import { useI18n } from "@/locales/i18n";
import { useSidebarWorkspaces } from "@/features/workspace/hooks/useSidebarWorkspaces";
import { CreateWorkspaceDialog } from "@/features/workspace/CreateWorkspaceDialog";
import { SidebarSessionList } from "@/features/chat/components/sidebar-session-list";
import type { AppPage, SettingsPage } from "@/types";
import { isSettingsPage } from "@/types";

const TOOLS_SUB_ITEMS: { id: "novels" | "trends"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "novels", labelKey: "novels", icon: BookOpenIcon },
  { id: "trends", labelKey: "scanTrends", icon: RadarIcon },
];

/**
 * 工作区文件视图下的二级菜单项。
 *
 * 这些功能依赖工作区上下文（项目目录 / 仓库 / 配置）,从「工具」菜单移到
 * 工作区下,通过「会话/文件」切换按钮显示。
 */
const WORKSPACE_FILE_ITEMS: { id: "loops" | "pipeline" | "git" | "audit" | "wiki"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "loops", labelKey: "loops", icon: CpuIcon },
  { id: "pipeline", labelKey: "pipeline", icon: WorkflowIcon },
  { id: "git", labelKey: "git", icon: GitBranchIcon },
  { id: "audit", labelKey: "audit", icon: ShieldCheckIcon },
  { id: "wiki", labelKey: "wiki", icon: NetworkIcon },
];

const SETTINGS_NAV_ITEMS: { id: SettingsPage; labelKey: string; icon: typeof GlobeIcon }[] = [
  { id: "settings.general", labelKey: "general", icon: GlobeIcon },
  { id: "settings.userProfile", labelKey: "userProfileLabel", icon: UserIcon },
  { id: "settings.genres", labelKey: "genresLabel", icon: BookMarkedIcon },
  { id: "settings.styles", labelKey: "stylesLabel", icon: PaletteIcon },
  { id: "settings.model", labelKey: "aiProvider", icon: CpuIcon },
  { id: "settings.embedding", labelKey: "embedding", icon: BoxesIcon },
  { id: "settings.prompts", labelKey: "prompts", icon: MessageSquareIcon },
  { id: "settings.agents", labelKey: "agents", icon: BotIcon },
  { id: "settings.bookSources", labelKey: "bookSources", icon: BookOpenIcon },
  { id: "settings.network", labelKey: "network", icon: NetworkIcon },
  { id: "settings.audit", labelKey: "audit", icon: ShieldIcon },
  { id: "settings.git", labelKey: "gitLabel", icon: GitBranchIcon },
  { id: "settings.shortcuts", labelKey: "shortcutsLabel", icon: KeyboardIcon },
  { id: "settings.system", labelKey: "systemLabel", icon: HardDriveIcon },
  { id: "settings.logs", labelKey: "logsLabel", icon: ScrollTextIcon },
  { id: "settings.skillEvolution", labelKey: "skillEvolutionLabel", icon: SparklesIcon },
  { id: "settings.learnedPreferences", labelKey: "learnedPreferencesLabel", icon: HeartIcon },
  { id: "settings.shortTermMemory", labelKey: "shortTermMemoryLabel", icon: BrainIcon },
  { id: "settings.agentAudit", labelKey: "agentAuditLabel", icon: ScrollIcon },
  { id: "settings.dailySummary", labelKey: "dailySummaryLabel", icon: CalendarClockIcon },
  { id: "settings.projectMemory", labelKey: "projectMemoryLabel", icon: FolderCogIcon },
  { id: "settings.toolLimits", labelKey: "toolLimitsLabel", icon: GaugeIcon },
  { id: "settings.about", labelKey: "aboutLabel", icon: InfoIcon },
];

export function AppSidebar() {
  const { currentPage } = useAppState();
  const dispatch = useAppDispatch();
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
  const [expandedTools, setExpandedTools] = useState<boolean>(false);
  /**
   * 每个工作区的视图模式：sessions（会话列表）/ files（文件功能菜单）。
   * 默认 sessions。切换按钮在 hover 时显示在工作区行右侧。
   */
  const [wsViewMode, setWsViewMode] = useState<Record<string, "sessions" | "files">>({});
  const isSettings = isSettingsPage(currentPage);

  function getWsView(wsId: string): "sessions" | "files" {
    return wsViewMode[wsId] ?? "sessions";
  }

  function toggleWsView(wsId: string) {
    setWsViewMode((prev) => ({
      ...prev,
      [wsId]: prev[wsId] === "files" ? "sessions" : "files",
    }));
  }

  function navigateTo(page: AppPage) {
    dispatch({ type: "SET_PAGE", payload: page });
  }

  return (
    <Sidebar collapsible="offcanvas">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              tooltip={t.app.name}
              onClick={() => navigateTo("trends")}
            >
              <div className="flex size-7 items-center justify-center rounded-[var(--radius-4)] bg-foreground text-background">
                <LayersIcon className="size-4" />
              </div>
              <span className="text-base font-semibold">{t.app.name}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        {isSettings ? (
          <SidebarGroup>
            <SidebarGroupLabel>{t.settings.title}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu className="gap-1">
                {SETTINGS_NAV_ITEMS.map((item) => (
                  <SidebarMenuItem key={item.id}>
                    <SidebarMenuButton
                      isActive={currentPage === item.id}
                      onClick={() => navigateTo(item.id)}
                    >
                      <item.icon />
                      <span>{t.settings[item.labelKey as keyof typeof t.settings] as string}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ) : (
          <>
            <SidebarGroup>
              <SidebarGroupLabel>{t.sidebar.home}</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu className="gap-1">
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={currentPage === "chat" || currentPage === "main-agent"}
                      onClick={() => {
                        // 仅进入空白页，不创建会话（输入消息时才创建）
                        useAgentStore.getState().clearCurrentSession();
                        navigateTo("chat");
                      }}
                      tooltip={t.sidebar.newTask}
                    >
                      <BotIcon />
                      <span>{t.sidebar.newTask}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <Collapsible
                    open={expandedTools}
                    onOpenChange={setExpandedTools}
                    className="group/tools"
                  >
                    <SidebarMenuItem>
                      <CollapsibleTrigger asChild>
                        <SidebarMenuButton
                          isActive={TOOLS_SUB_ITEMS.some((item) => item.id === currentPage)}
                          tooltip={t.sidebar.tools}
                        >
                          <WrenchIcon className="group-hover/tools:hidden" />
                          <ChevronRightIcon
                            className={cn(
                              "hidden group-hover/tools:block transition-transform duration-200",
                              expandedTools && "rotate-90",
                            )}
                          />
                          <span>{t.sidebar.tools}</span>
                        </SidebarMenuButton>
                      </CollapsibleTrigger>
                      <CollapsibleContent>
                        <SidebarMenuSub>
                          {TOOLS_SUB_ITEMS.map((item) => (
                            <SidebarMenuSubItem key={item.id}>
                              <SidebarMenuSubButton
                                isActive={currentPage === item.id}
                                onClick={() => navigateTo(item.id)}
                              >
                                <item.icon />
                                <span>{t.sidebar[item.labelKey as keyof typeof t.sidebar] as string}</span>
                              </SidebarMenuSubButton>
                            </SidebarMenuSubItem>
                          ))}
                        </SidebarMenuSub>
                      </CollapsibleContent>
                    </SidebarMenuItem>
                  </Collapsible>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={currentPage === "skills"}
                      onClick={() => navigateTo("skills")}
                      tooltip={t.sidebar.skills}
                    >
                      <PuzzleIcon />
                      <span>{t.sidebar.skills}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={currentPage === "dashboard"}
                      onClick={() => navigateTo("dashboard")}
                      tooltip={t.dashboard.title}
                    >
                      <TrendingUpIcon />
                      <span>{t.dashboard.title}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={currentPage === "knowledge"}
                      onClick={() => navigateTo("knowledge")}
                      tooltip={t.sidebar.knowledge}
                    >
                      <BookMarkedIcon />
                      <span>{t.sidebar.knowledge}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>

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
                    workspaces.map((ws) => {
                      const isActive = activeWorkspaceId === ws.id;
                      const isExpanded = expandedWs === ws.id;
                      const viewMode = getWsView(ws.id);
                      return (
                        <Collapsible
                          key={ws.id}
                          open={isExpanded}
                          onOpenChange={(open) => {
                            setExpandedWs(open ? ws.id : null);
                            setActiveWorkspace(ws.id);
                            if (open && viewMode === "sessions") {
                              navigateTo("chat");
                            }
                          }}
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
                                  toggleWsView(ws.id);
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
                                  removeWorkspace(ws.id);
                                }}
                              >
                                <MoreHorizontalIcon className="size-4" />
                              </button>
                            </div>
                            <CollapsibleContent>
                              {viewMode === "sessions" ? (
                                <SidebarSessionList workspaceId={ws.id} />
                              ) : (
                                <SidebarMenuSub>
                                  {WORKSPACE_FILE_ITEMS.map((item) => (
                                    <SidebarMenuSubItem key={item.id}>
                                      <SidebarMenuSubButton
                                        isActive={currentPage === item.id}
                                        onClick={() => navigateTo(item.id)}
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
                    })
                  )}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </>
        )}
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            {isSettings ? (
              <SidebarMenuButton onClick={() => navigateTo("trends")}>
                <ArrowLeftIcon />
                <span>{t.sidebar.backToWorkspace}</span>
              </SidebarMenuButton>
            ) : (
              <SidebarMenuButton onClick={() => navigateTo("settings.general")}>
                <SettingsIcon />
                <span>{t.sidebar.settings}</span>
              </SidebarMenuButton>
            )}
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
