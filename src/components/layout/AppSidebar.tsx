/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AppSidebar - 应用侧边栏组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback } from "react";
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
  ArrowLeftIcon,
  GlobeIcon,
  ShieldIcon,
  ChevronRightIcon,
  BookOpenIcon,
  BookMarkedIcon,
  PuzzleIcon,
  BotIcon,
  WrenchIcon,
  CpuIcon,
  NetworkIcon,
  RadarIcon,
  UserIcon,
  FilesIcon,
  PaletteIcon,
  BrainIcon,
  BarChart3Icon,
  InfoIcon,
  FolderIcon,
  GraduationCapIcon,
  CalendarIcon,
  ActivityIcon,
  GitBranchIcon,
  LockIcon,
  MessageSquareIcon,
  BoxesIcon,
  ArchiveIcon,
} from "lucide-react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { useI18n } from "@/locales/i18n";
import type { AppPage, SettingsPage } from "@/types";
import { isSettingsPage } from "@/types";
import { SidebarHeaderNav } from "./sidebar-header";
import { SessionSection } from "./session-section";

// ── 常量配置 ────────────────────────────────────────────────────────────────

const TOOLS_SUB_ITEMS: { id: "novels" | "trends"; labelKey: string; icon: typeof BookOpenIcon }[] = [
  { id: "novels", labelKey: "novels", icon: BookOpenIcon },
  { id: "trends", labelKey: "scanTrends", icon: RadarIcon },
];

const SETTINGS_NAV_ITEMS: { id: SettingsPage; labelKey: string; icon: typeof GlobeIcon }[] = [
  // ── 基础设置组 ────────────────────────────────────────────────────────
  { id: "settings.general", labelKey: "general", icon: GlobeIcon },
  { id: "settings.system", labelKey: "system", icon: SettingsIcon },
  { id: "settings.shortcuts", labelKey: "shortcuts", icon: WrenchIcon },
  { id: "settings.about", labelKey: "about", icon: InfoIcon },
  // ── 用户与内容组 ────────────────────────────────────────────────────────
  { id: "settings.userProfile", labelKey: "userProfileLabel", icon: UserIcon },
  { id: "settings.genres", labelKey: "genresLabel", icon: BookMarkedIcon },
  { id: "settings.styles", labelKey: "stylesLabel", icon: PaletteIcon },
  // ── AI 模型组 ────────────────────────────────────────────────────────
  { id: "settings.model", labelKey: "model", icon: CpuIcon },
  { id: "settings.embedding", labelKey: "embedding", icon: BoxesIcon },
  { id: "settings.prompts", labelKey: "prompts", icon: MessageSquareIcon },
  { id: "settings.agents", labelKey: "agents", icon: BotIcon },
  // ── 内容与记忆组 ────────────────────────────────────────────────────────
  { id: "settings.bookSources", labelKey: "bookSources", icon: BookOpenIcon },
  { id: "settings.shortTerm", labelKey: "shortTermMemoryLabel", icon: BrainIcon },
  { id: "settings.project", labelKey: "projectLabel", icon: FolderIcon },
  { id: "settings.learned", labelKey: "learnedLabel", icon: GraduationCapIcon },
  { id: "settings.daily", labelKey: "dailyLabel", icon: CalendarIcon },
  { id: "settings.skillMemory", labelKey: "skillLabel", icon: PuzzleIcon },
  // ── 安全与网络组 ────────────────────────────────────────────────────────
  { id: "settings.rules", labelKey: "rulesLabel", icon: ShieldIcon },
  { id: "settings.events", labelKey: "eventsLabel", icon: ActivityIcon },
  { id: "settings.network", labelKey: "networkLabel", icon: NetworkIcon },
  { id: "settings.git", labelKey: "gitLabel", icon: GitBranchIcon },
  { id: "settings.limits", labelKey: "limitsLabel", icon: LockIcon },
  // ── 统计 ────────────────────────────────────────────────────────
  { id: "settings.usageStats", labelKey: "usageStatsLabel", icon: BarChart3Icon },
  { id: "settings.archive", labelKey: "archiveLabel", icon: ArchiveIcon },
];

// ── 侧边栏组件 ──────────────────────────────────────────────────────────────

export function AppSidebar() {
  const { currentPage } = useAppState();
  const dispatch = useAppDispatch();
  const { t } = useI18n();
  const [expandedTools, setExpandedTools] = useState<boolean>(false);
  const isSettings = isSettingsPage(currentPage);

  const navigateTo = useCallback((page: AppPage) => {
    dispatch({ type: "SET_PAGE", payload: page });
  }, [dispatch]);

  // 处理新建任务按钮点击
  const handleNewTaskClick = useCallback(async () => {
    const { useAgentStore } = await import("@/features/chat/store");
    const store = useAgentStore.getState();

    if (store.isAgentRunning()) {
      // Agent 正在运行，显示确认对话框（使用 Tauri 原生对话框）
      const { ask } = await import("@tauri-apps/plugin-dialog");
      const confirmed = await ask(
        t.chat.agentRunningDialog.description,
        {
          title: t.chat.agentRunningDialog.title,
          kind: "warning",
          okLabel: t.chat.agentRunningDialog.interruptAndContinue,
          cancelLabel: t.common.cancel,
        }
      );

      if (confirmed) {
        store.forceStop();
        store.clearCurrentSession();
        navigateTo("chat");
      }
    } else {
      store.clearCurrentSession();
      navigateTo("chat");
    }
  }, [navigateTo, t]);

  return (
    <Sidebar collapsible="offcanvas">
      <SidebarHeader>
        <SidebarHeaderNav onNavigate={navigateTo} />
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
                      onClick={handleNewTaskClick}
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
                      isActive={currentPage === "editor"}
                      onClick={() => navigateTo("editor")}
                      tooltip={t.sidebar.editor}
                    >
                      <FilesIcon />
                      <span>{t.sidebar.editor}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>

            <SessionSection currentPage={currentPage} onNavigate={navigateTo} />
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
