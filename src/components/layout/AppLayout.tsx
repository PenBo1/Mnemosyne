/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AppLayout - 应用布局组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { SidebarProvider } from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AppSidebar } from "./AppSidebar";
import { Router } from "@/routes";
import { useShortcut } from "@/lib/shortcut-dispatcher";
import { useAppDispatch } from "@/lib/app-context";

// ── 全局快捷键 ──────────────────────────────────────────────────────────────

/** 注册全局快捷键 handler */
function useGlobalShortcuts() {
  const dispatch = useAppDispatch();

  useShortcut("newChat", () => {
    dispatch({ type: "SET_PAGE", payload: "chat" });
    // 仅进入空白页，不创建会话（输入消息时才创建）
    // 动态 import 避免 layout 静态依赖 @/features/chat/store
    void import("@/features/chat/store").then((m) =>
      m.useAgentStore.getState().clearCurrentSession()
    );
  });

  useShortcut("save", () => {
    // 阻止浏览器默认保存，未来接入编辑器保存逻辑
  });
}

// ── 布局组件 ────────────────────────────────────────────────────────────────

export function AppLayout() {
  useGlobalShortcuts();
  return (
    <TooltipProvider>
      <SidebarProvider className="h-screen">
        <AppSidebar />
        <main className="flex-1 h-full">
          <Router />
        </main>
      </SidebarProvider>
    </TooltipProvider>
  );
}
