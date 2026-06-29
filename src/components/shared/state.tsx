import * as React from "react"
import { cn } from "@/shared/utils"
import { Spinner } from "@/components/ui/spinner"
import {
  Empty,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  EmptyDescription,
  EmptyContent,
} from "@/components/ui/empty"

/**
 * LoadingState — 统一加载态
 *
 * 替代各页面 4 种不同的加载写法（Skeleton / Spinner / "Loading..." / t.common.loading）。
 */
function LoadingState({
  className,
  label,
}: {
  className?: string
  label?: string
}) {
  return (
    <div
      className={cn(
        "flex flex-1 items-center justify-center gap-2 py-12 text-muted-foreground",
        className
      )}
    >
      <Spinner className="size-5" />
      {label && <span className="text-sm">{label}</span>}
    </div>
  )
}

/**
 * EmptyState — 统一空态
 *
 * 封装 shadcn Empty 组件，提供图标+标题+描述+操作的标准化空态。
 * 替代各页面 5 种不同的空态写法。
 */
function EmptyState({
  icon,
  title,
  description,
  children,
  className,
}: {
  icon?: React.ReactNode
  title: string
  description?: string
  children?: React.ReactNode
  className?: string
}) {
  return (
    <Empty className={cn("py-12", className)}>
      <EmptyHeader>
        {icon && <EmptyMedia variant="icon">{icon}</EmptyMedia>}
        <EmptyTitle>{title}</EmptyTitle>
        {description && <EmptyDescription>{description}</EmptyDescription>}
      </EmptyHeader>
      {children && <EmptyContent>{children}</EmptyContent>}
    </Empty>
  )
}

export { LoadingState, EmptyState }
