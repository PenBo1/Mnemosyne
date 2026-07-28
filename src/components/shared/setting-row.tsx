/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SettingRow - 统一设置行组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"
import { memo } from "react"
import { cn } from "@/lib/utils"

// ── 设置行组件 ──────────────────────────────────────────────────────────────

/**
 * 统一设置行
 * 左侧 label + description，右侧 control
 */
const SettingRow = memo(function SettingRow({
  label,
  description,
  children,
  className,
}: {
  label: React.ReactNode
  description?: React.ReactNode
  children?: React.ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-4 border-b py-3 last:border-b-0",
        className
      )}
    >
      <div className="flex flex-col gap-0.5">
        <span className="text-sm font-medium">{label}</span>
        {description && (
          <span className="text-xs text-muted-foreground">{description}</span>
        )}
      </div>
      {children && <div className="shrink-0">{children}</div>}
    </div>
  )
})

export { SettingRow }
