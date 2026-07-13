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
  SidebarMenuAction,
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
  ChevronDownIcon,
  GitBranchIcon,
  Trash2Icon,
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
} from "lucide-react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { useAgentStore } from "@/features/chat/store";
import { useI18n } from "@/locales/i18n";
import { useSidebarWorkspaces } from "@/features/workspace/hooks/useSidebarWorkspaces";
import { CreateWorkspaceDialog } from "@/features/workspace/CreateWorkspaceDialog";
import { SidebarSessionList } from "@/features/chat/components/sidebar-session-list";
import type { AppPage, SettingsPage } from "@/types";
import { isSettingsPage } from "@/types";

const TOOLS_SUB_ITEMS: { id: "novels" | "trends" | "loops" | "git" | "pipeline"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "novels", labelKey: "novels", icon: BookOpenIcon },
  { id: "trends", labelKey: "scanTrends", icon: TrendingUpIcon },
  { id: "loops", labelKey: "loops", icon: CpuIcon },
  { id: "pipeline", labelKey: "pipeline", icon: WorkflowIcon },
  { id: "git", labelKey: "git", icon: GitBranchIcon },
];

const SETTINGS_NAV_ITEMS: { id: SettingsPage; labelKey: string; icon: typeof GlobeIcon }[] = [
  { id: "settings.general", labelKey: "general", icon: GlobeIcon },
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
  const isSettings = isSettingsPage(currentPage);

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
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={currentPage === "wiki"}
                      onClick={() => navigateTo("wiki")}
                      tooltip={t.sidebar.wiki}
                    >
                      <NetworkIcon />
                      <span>{t.sidebar.wiki}</span>
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
                      return (
                        <Collapsible
                          key={ws.id}
                          open={isExpanded}
                          onOpenChange={(open) => {
                            setExpandedWs(open ? ws.id : null);
                            setActiveWorkspace(ws.id);
                            if (open) {
                              // 展开工作区时导航到对话页（展示该工作区的会话列表）
                              navigateTo("chat");
                            }
                          }}
                        >
                          <SidebarMenuItem>
                            <CollapsibleTrigger asChild>
                              <SidebarMenuButton isActive={isActive}>
                                <FolderIcon />
                                <span className="flex-1 truncate">{ws.name}</span>
                                {isExpanded ? <ChevronDownIcon /> : <ChevronRightIcon />}
                              </SidebarMenuButton>
                            </CollapsibleTrigger>
                            <SidebarMenuAction
                              showOnHover
                              onClick={(e) => {
                                e.stopPropagation();
                                removeWorkspace(ws.id);
                              }}
                            >
                              <Trash2Icon />
                            </SidebarMenuAction>
                            <CollapsibleContent>
                              <SidebarSessionList workspaceId={ws.id} />
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
