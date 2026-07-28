/**
 * ═══════════════════════════════════════════════════════════════════════════
 * StatCard - 审计页统计卡片组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: number;
  subtitle?: string;
  subtitleClass?: string;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 统计卡片组件，用于展示审计页的统计数据
 */
export const StatCard = memo(function StatCard({ icon, label, value, subtitle, subtitleClass }: StatCardProps) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-center gap-2">
        {icon}
        <div className="text-xs text-muted-foreground">{label}</div>
      </div>
      <div className="mt-2 text-2xl font-semibold">{value.toLocaleString()}</div>
      {subtitle && (
        <div className={`text-xs ${subtitleClass ?? "text-muted-foreground"}`}>
          {subtitle}
        </div>
      )}
    </div>
  );
});