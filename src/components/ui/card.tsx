/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Card - 卡片组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"

import { cn } from "@/lib/utils"

// ── 卡片组件 ────────────────────────────────────────────────────────────────

function Card({
  className,
  size = "default",
  ...props
}: React.ComponentProps<"div"> & { size?: "default" | "sm" }) {
  return (
    <div
      data-slot="card"
      data-size={size}
      className={cn(
        "group/card flex flex-col gap-(--card-spacing) overflow-hidden rounded-[var(--radius-4)] bg-card py-(--card-spacing) text-xs/relaxed text-card-foreground ring-1 ring-[var(--border-neutral-l1)] [--card-spacing:16px] has-[>img:first-child]:pt-0 data-[size=sm]:[--card-spacing:12px] *:[img:first-child]:rounded-t-[var(--radius-4)] *:[img:last-child]:rounded-b-[var(--radius-4)]",
        className
      )}
      {...props}
    />
  )
}

// ── 卡片头部组件 ────────────────────────────────────────────────────────────

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn(
        "group/card-header @container/card-header grid auto-rows-min items-start gap-1 rounded-t-lg px-(--card-spacing) has-data-[slot=card-action]:grid-cols-[1fr_auto] has-data-[slot=card-description]:grid-rows-[auto_auto] [.border-b]:pb-(--card-spacing)",
        className
      )}
      {...props}
    />
  )
}

// ── 卡片标题组件 ────────────────────────────────────────────────────────────

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-title"
      className={cn("font-heading text-sm font-medium", className)}
      {...props}
    />
  )
}

// ── 卡片描述组件 ────────────────────────────────────────────────────────────

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-description"
      className={cn("text-xs/relaxed text-muted-foreground", className)}
      {...props}
    />
  )
}

// ── 卡片操作组件 ────────────────────────────────────────────────────────────

function CardAction({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-action"
      className={cn(
        "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
        className
      )}
      {...props}
    />
  )
}

// ── 卡片内容组件 ────────────────────────────────────────────────────────────

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-content"
      className={cn("px-(--card-spacing)", className)}
      {...props}
    />
  )
}

// ── 卡片底部组件 ────────────────────────────────────────────────────────────

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-footer"
      className={cn(
        "flex items-center rounded-b-lg px-(--card-spacing) [.border-t]:pt-(--card-spacing)",
        className
      )}
      {...props}
    />
  )
}

export {
  Card,
  CardHeader,
  CardFooter,
  CardTitle,
  CardAction,
  CardDescription,
  CardContent,
}
