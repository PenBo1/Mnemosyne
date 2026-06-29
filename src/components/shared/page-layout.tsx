import * as React from "react"
import { cn } from "@/shared/utils"

/**
 * PageContainer — 统一页面外壳
 *
 * 提供统一的 padding、滚动、最大宽度控制。
 * 所有页面应使用此组件作为最外层容器。
 */
function PageContainer({
  className,
  scrollable = true,
  ...props
}: React.ComponentProps<"div"> & { scrollable?: boolean }) {
  return (
    <div
      data-slot="page-container"
      className={cn(
        "flex h-full flex-col gap-6 p-6",
        scrollable && "overflow-y-auto",
        className
      )}
      {...props}
    />
  )
}

/**
 * PageHeader — 统一页面头部
 *
 * 标准化标题、描述、右侧操作区的布局。
 * 消除各页面 5 种不同的头部写法。
 */
function PageHeader({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="page-header"
      className={cn(
        "flex flex-wrap items-center justify-between gap-4",
        className
      )}
      {...props}
    />
  )
}

function PageHeading({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="page-heading"
      className={cn("flex flex-col gap-1", className)}
      {...props}
    />
  )
}

function PageTitle({
  className,
  ...props
}: React.ComponentProps<"h1">) {
  return (
    <h1
      data-slot="page-title"
      className={cn(
        "flex items-center gap-2 text-2xl font-bold tracking-tight",
        className
      )}
      {...props}
    />
  )
}

function PageDescription({
  className,
  ...props
}: React.ComponentProps<"p">) {
  return (
    <p
      data-slot="page-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  )
}

function PageActions({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="page-actions"
      className={cn("flex items-center gap-2", className)}
      {...props}
    />
  )
}

export {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
}
