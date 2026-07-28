/**
 * ═══════════════════════════════════════════════════════════════════════════
 * State - 状态展示组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"
import { memo } from "react"
import { cn } from "@/lib/utils"
import { Spinner } from "@/components/ui/spinner"
import { Badge } from "@/components/ui/badge"
import {
  Empty,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  EmptyDescription,
  EmptyContent,
} from "@/components/ui/empty"

// ── 加载状态组件 ────────────────────────────────────────────────────────────

const LoadingState = memo(function LoadingState({
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
})

// ── 空状态组件 ──────────────────────────────────────────────────────────────

const EmptyState = memo(function EmptyState({
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
})

// ── 状态徽章组件 ────────────────────────────────────────────────────────────

const StatusBadge = memo(function StatusBadge({
  variant = "default",
  children,
  className,
}: {
  variant?: "default" | "secondary" | "destructive" | "outline" | "success" | "warning" | "info"
  children: React.ReactNode
  className?: string
}) {
  return (
    <Badge variant={variant} className={className}>
      {children}
    </Badge>
  )
})

export { LoadingState, EmptyState, StatusBadge }