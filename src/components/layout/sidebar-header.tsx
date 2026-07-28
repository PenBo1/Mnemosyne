/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SidebarHeaderNav - 侧边栏顶部导航组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { SidebarMenu, SidebarMenuItem, SidebarMenuButton } from "@/components/ui/sidebar";
import { LayersIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { AppPage } from "@/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface SidebarHeaderNavProps {
  onNavigate: (page: AppPage) => void;
}

// ── 侧边栏头部导航组件 ──────────────────────────────────────────────────────

/** 侧边栏顶部 logo + 应用名称，点击跳转趋势页 */
export function SidebarHeaderNav({ onNavigate }: SidebarHeaderNavProps) {
  const { t } = useI18n();
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton
          size="lg"
          tooltip={t.app.name}
          onClick={() => onNavigate("trends")}
        >
          <div className="flex size-7 items-center justify-center rounded-[var(--radius-4)] bg-foreground text-background">
            <LayersIcon className="size-4" />
          </div>
          <span className="text-base font-semibold">{t.app.name}</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
