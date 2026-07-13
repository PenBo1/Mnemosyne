import * as React from "react"
import { cn } from "@/lib/utils"

/**
 * SettingRow — 统一设置行
 *
 * 替代 settings/ 8 个文件中重复的
 * `CardContent + flex items-center justify-between border-b py-3` 模式。
 * 左侧 label + description，右侧 control。
 */
function SettingRow({
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
}

export { SettingRow }
