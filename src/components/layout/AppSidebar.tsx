import { useState, useEffect } from "react";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
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
  PlusIcon,
  Trash2Icon,
  MessageSquareIcon,
  TrendingUpIcon,
  BookOpenIcon,
  BookMarkedIcon,
  PuzzleIcon,
  BotIcon,
  SparklesIcon,
  WrenchIcon,
  FolderOpenIcon,
  CpuIcon,
} from "lucide-react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/lib/i18n";
import { pickDirectory } from "@/services/workspaces";
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
  { id: "audit", labelKey: "audit", icon: ShieldIcon },
  { id: "system", labelKey: "system", icon: ShieldCheckIcon },
];

export function AppSidebar() {
  const { currentPage, settingsTab } = useAppState();
  const dispatch = useAppDispatch();
  const { workspaces, activeWorkspaceId, loadWorkspaces, addWorkspace, removeWorkspace, setActiveWorkspace } =
    useWorkspaceStore();
  const { t } = useI18n();
  const [expandedWs, setExpandedWs] = useState<string | null>(null);
  const [expandedTools, setExpandedTools] = useState<boolean>(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newWorkspaceName, setNewWorkspaceName] = useState("");
  const [newWorkspacePath, setNewWorkspacePath] = useState("");
  const [creating, setCreating] = useState(false);
  const isSettings = currentPage === "settings";

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  function navigateTo(page: AppPage) {
    dispatch({ type: "SET_PAGE", payload: page });
  }

  function setSettingsTab(tab: SettingsTab) {
    dispatch({ type: "SET_SETTINGS_TAB", payload: tab });
  }

  async function handlePickDirectory() {
    const selected = await pickDirectory();
    if (selected) {
      setNewWorkspacePath(selected);
      if (!newWorkspaceName) {
        const folderName = selected.split(/[\\/]/).pop() || "";
        setNewWorkspaceName(folderName);
      }
    }
  }

  async function handleAddWorkspace() {
    if (!newWorkspaceName.trim() || !newWorkspacePath) return;
    setCreating(true);
    try {
      await addWorkspace(newWorkspaceName.trim(), newWorkspacePath);
      setDialogOpen(false);
      setNewWorkspaceName("");
      setNewWorkspacePath("");
    } catch (err) {
      console.error("Failed to create workspace:", err);
    } finally {
      setCreating(false);
    }
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
                      <PlusIcon className="ml-auto size-3.5 opacity-50" />
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
                <Dialog open={dialogOpen} onOpenChange={(open) => {
                  setDialogOpen(open);
                  if (!open) {
                    setNewWorkspaceName("");
                    setNewWorkspacePath("");
                  }
                }}>
                  <DialogTrigger asChild>
                    <button className="rounded-md p-0.5 hover:bg-sidebar-accent">
                      <PlusIcon className="size-3.5" />
                    </button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>{t.sidebar.createWorkspace}</DialogTitle>
                      <DialogDescription>{t.sidebar.createWorkspaceDesc}</DialogDescription>
                    </DialogHeader>
                    <FieldGroup>
                      <Field>
                        <FieldLabel>Name</FieldLabel>
                        <Input
                          value={newWorkspaceName}
                          onChange={(e) => setNewWorkspaceName(e.target.value)}
                          placeholder={t.sidebar.workspaceNamePlaceholder}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" && newWorkspacePath) handleAddWorkspace();
                          }}
                        />
                      </Field>
                      <Field>
                        <FieldLabel>Directory</FieldLabel>
                        <div className="flex gap-2">
                          <Input
                            value={newWorkspacePath}
                            onChange={(e) => setNewWorkspacePath(e.target.value)}
                            placeholder="Select a directory..."
                            readOnly
                          />
                          <Button variant="outline" onClick={handlePickDirectory} type="button">
                            <FolderOpenIcon />
                          </Button>
                        </div>
                      </Field>
                    </FieldGroup>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setDialogOpen(false)}>
                        {t.common.cancel}
                      </Button>
                      <Button onClick={handleAddWorkspace} disabled={!newWorkspaceName.trim() || !newWorkspacePath || creating}>
                        {t.common.create}
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>
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
