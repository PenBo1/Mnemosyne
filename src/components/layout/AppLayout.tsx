import { SidebarProvider } from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AppSidebar } from "./AppSidebar";
import { Router } from "@/routes";
import { useShortcut } from "@/lib/shortcut-dispatcher";
import { useAppDispatch } from "@/lib/app-context";
import { useAgentStore } from "@/features/chat/store";

/** 注册全局快捷键 handler */
function useGlobalShortcuts() {
  const dispatch = useAppDispatch();

  useShortcut("newChat", () => {
    dispatch({ type: "SET_PAGE", payload: "chat" });
    // 仅进入空白页，不创建会话（输入消息时才创建）
    useAgentStore.getState().clearCurrentSession();
  });

  useShortcut("save", () => {
    // 阻止浏览器默认保存，未来接入编辑器保存逻辑
  });
}

export function AppLayout() {
  useGlobalShortcuts();
  return (
    <TooltipProvider>
      <SidebarProvider className="h-screen">
        <AppSidebar />
        <main className="flex-1 h-full overflow-hidden">
          <Router />
        </main>
      </SidebarProvider>
    </TooltipProvider>
  );
}
