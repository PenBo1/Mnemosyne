/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SettingsSection - 设置区块组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface SettingsSectionProps {
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
}

interface SettingsRowProps {
  label: string;
  description?: string;
  children: ReactNode;
  className?: string;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 设置区块容器，包含标题和内容区域
 */
function SettingsSection({
  title,
  description,
  children,
  className,
}: SettingsSectionProps) {
  return (
    <Card className={cn("py-0", className)}>
      <CardContent className="p-0">
        {/* ── 标题区域 ──────────────────────────────────────────────────────── */}
        <div className="px-4 py-3">
          <h3 className="text-sm font-medium">{title}</h3>
          {description && (
            <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
          )}
        </div>
        <Separator />
        {/* ── 内容区域 ──────────────────────────────────────────────────────── */}
        <div className="p-0">{children}</div>
      </CardContent>
    </Card>
  );
}

/**
 * 设置行组件，包含标签和控件
 */
function SettingsRow({
  label,
  description,
  children,
  className,
}: SettingsRowProps) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-4 px-4 py-3 border-b last:border-b-0",
        className
      )}
    >
      <div className="flex flex-col gap-0.5">
        <span className="text-sm font-medium">{label}</span>
        {description && (
          <span className="text-xs text-muted-foreground">{description}</span>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export { SettingsSection, SettingsRow };