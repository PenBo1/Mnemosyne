import * as React from "react"
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

function StatusBadge({
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
}

export { LoadingState, EmptyState, StatusBadge }