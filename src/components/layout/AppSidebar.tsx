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
  { id: "settings.general", labelKey: "general", icon: GlobeIcon },
  { id: "settings.userProfile", labelKey: "userProfileLabel", icon: UserIcon },
  { id: "settings.genres", labelKey: "genresLabel", icon: BookMarkedIcon },
  { id: "settings.styles", labelKey: "stylesLabel", icon: PaletteIcon },
  { id: "settings.ai", labelKey: "aiProvider", icon: CpuIcon },
  { id: "settings.bookSources", labelKey: "bookSources", icon: BookOpenIcon },
  { id: "settings.memory", labelKey: "memoryLabel", icon: BrainIcon },
  { id: "settings.security", labelKey: "audit", icon: ShieldIcon },
  { id: "settings.networkTools", labelKey: "network", icon: NetworkIcon },
  { id: "settings.usageStats", labelKey: "usageStatsLabel", icon: BarChart3Icon },
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
                      onClick={() => {
                        // 仅进入空白页，不创建会话（输入消息时才创建）
                        // 动态 import 避免 layout 静态依赖 @/features/chat/store
                        void import("@/features/chat/store").then((m) =>
                          m.useAgentStore.getState().clearCurrentSession()
                        );
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
