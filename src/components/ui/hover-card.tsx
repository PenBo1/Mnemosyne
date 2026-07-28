/**
 * ═══════════════════════════════════════════════════════════════════════════
 * HoverCard - 悬浮卡片组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"

import { cn } from "@/lib/utils"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"

// ── 悬浮卡片组件 ────────────────────────────────────────────────────────────

function HoverCard({
  ...props
}: React.ComponentProps<typeof Popover>) {
  return (
    <Popover
      openDelay={200}
      closeDelay={100}
      {...props}
    />
  )
}

// ── 悬浮卡片触发器组件 ──────────────────────────────────────────────────────

function HoverCardTrigger({
  ...props
}: React.ComponentProps<typeof PopoverTrigger>) {
  return (
    <PopoverTrigger
      asChild
      {...props}
    />
  )
}

// ── 悬浮卡片内容组件 ────────────────────────────────────────────────────────

function HoverCardContent({
  className,
  align = "center",
  sideOffset = 4,
  ...props
}: React.ComponentProps<typeof PopoverContent>) {
  return (
    <PopoverContent
      align={align}
      sideOffset={sideOffset}
      className={cn(
        "w-60 rounded-lg bg-popover p-4 text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
        className
      )}
      {...props}
    />
  )
}

export { HoverCard, HoverCardTrigger, HoverCardContent }