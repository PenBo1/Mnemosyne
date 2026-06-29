import { SidebarProvider } from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AppSidebar } from "./AppSidebar";
import { Router } from "@/routes";

export function AppLayout() {
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
