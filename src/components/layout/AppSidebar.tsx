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
  SettingsIcon,
  LayersIcon,
  ArrowLeftIcon,
  GlobeIcon,
  ShieldCheckIcon,
  ShieldIcon,
  FolderIcon,
  ChevronRightIcon,
  ChevronDownIcon,
  FileTextIcon,
  UsersIcon,
  GitBranchIcon,
  ClockIcon,
  BookmarkIcon,
  Trash2Icon,
  MessageSquareIcon,
  TrendingUpIcon,
  BookOpenIcon,
  BookMarkedIcon,
  PuzzleIcon,
  BotIcon,
  SparklesIcon,
  WrenchIcon,
  CpuIcon,
} from "lucide-react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { useI18n } from "@/lib/i18n";
import { useSidebarWorkspaces } from "@/hooks/useSidebarWorkspaces";
import { CreateWorkspaceDialog } from "./CreateWorkspaceDialog";
import type { AppPage, SettingsTab, WorkspacePage } from "@/types";

const WORKSPACE_SUB_ITEMS: { id: WorkspacePage; labelKey: string; icon: typeof FileTextIcon }[] = [
  { id: "overview", labelKey: "overview", icon: BookOpenIcon },
  { id: "characters", labelKey: "characters", icon: UsersIcon },
  { id: "worldbuilding", labelKey: "worldbuilding", icon: GlobeIcon },
  { id: "plot", labelKey: "plot", icon: GitBranchIcon },
  { id: "timeline", labelKey: "timeline", icon: ClockIcon },
  { id: "research", labelKey: "research", icon: BookmarkIcon },
];

const TOOLS_SUB_ITEMS: { id: "novels" | "trends"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "novels", labelKey: "novels", icon: BookOpenIcon },
  { id: "trends", labelKey: "scanTrends", icon: TrendingUpIcon },
];

const SETTINGS_NAV_ITEMS: { id: SettingsTab; labelKey: string; icon: typeof GlobeIcon }[] = [
  { id: "general", labelKey: "general", icon: GlobeIcon },
  { id: "model", labelKey: "aiProvider", icon: CpuIcon },
  { id: "prompts", labelKey: "prompts", icon: MessageSquareIcon },
  { id: "agents", labelKey: "agents", icon: BotIcon },
  { id: "bookSources", labelKey: "bookSources", icon: BookOpenIcon },
  { id: "audit", labelKey: "audit", icon: ShieldIcon },
  { id: "system", labelKey: "system", icon: ShieldCheckIcon },
];

export function AppSidebar() {
  const { currentPage, settingsTab } = useAppState();
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
  const isSettings = currentPage === "settings";

  function navigateTo(page: AppPage) {
    dispatch({ type: "SET_PAGE", payload: page });
  }

  function setSettingsTab(tab: SettingsTab) {
    dispatch({ type: "SET_SETTINGS_TAB", payload: tab });
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
              <div className="flex size-7 items-center justify-center rounded-lg bg-foreground text-background">
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
                      isActive={settingsTab === item.id}
                      onClick={() => setSettingsTab(item.id)}
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
                      isActive={currentPage === "chat"}
                      onClick={() => navigateTo("chat")}
                      tooltip={t.chat.title}
                    >
                      <SparklesIcon />
                      <span>{t.sidebar.newChat}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      isActive={TOOLS_SUB_ITEMS.some((item) => item.id === currentPage)}
                      onClick={() => setExpandedTools(!expandedTools)}
                      tooltip={t.sidebar.tools}
                    >
                      <WrenchIcon />
                      <span>{t.sidebar.tools}</span>
                      {expandedTools ? (
                        <ChevronDownIcon className="ml-auto size-3.5" />
                      ) : (
                        <ChevronRightIcon className="ml-auto size-3.5" />
                      )}
                    </SidebarMenuButton>
                    {expandedTools && (
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
                    )}
                  </SidebarMenuItem>
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
                      return (
                        <SidebarMenuItem key={ws.id}>
                          <SidebarMenuButton
                            isActive={isActive}
                            onClick={() => {
                              setActiveWorkspace(ws.id);
                              setExpandedWs(isExpanded ? null : ws.id);
                              if (!isExpanded) {
                                navigateTo("novels");
                              }
                            }}
                          >
                            <FolderIcon />
                            <span className="flex-1 truncate">{ws.name}</span>
                            <span
                              role="button"
                              tabIndex={0}
                              onClick={(e) => {
                                e.stopPropagation();
                                removeWorkspace(ws.id);
                              }}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                  e.stopPropagation();
                                  removeWorkspace(ws.id);
                                }
                              }}
                              className="rounded-md p-0.5 opacity-0 group-hover/menu-item:opacity-100 hover:bg-destructive/10 hover:text-destructive cursor-pointer"
                            >
                              <Trash2Icon className="size-3" />
                            </span>
                            {isExpanded ? (
                              <ChevronDownIcon className="size-3.5" />
                            ) : (
                              <ChevronRightIcon className="size-3.5" />
                            )}
                          </SidebarMenuButton>
                          {isExpanded && (
                            <SidebarMenuSub>
                              {WORKSPACE_SUB_ITEMS.map((item) => (
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
                        </SidebarMenuItem>
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
              <SidebarMenuButton onClick={() => navigateTo("settings")}>
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
