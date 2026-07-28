/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Popover - 弹出框组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"
import { Popover as PopoverPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

// ── 弹出框组件 ──────────────────────────────────────────────────────────────

function Popover({
  openDelay,
  closeDelay,
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Root> & {
  openDelay?: number
  closeDelay?: number
}) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />
}

// ── 弹出框触发器组件 ────────────────────────────────────────────────────────

function PopoverTrigger({
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />
}

// ── 弹出框锚点组件 ──────────────────────────────────────────────────────────

function PopoverAnchor({
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Anchor>) {
  return <PopoverPrimitive.Anchor data-slot="popover-anchor" {...props} />
}

// ── 弹出框内容组件 ──────────────────────────────────────────────────────────

function PopoverContent({
  className,
  align = "center",
  sideOffset = 4,
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Content>) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        align={align}
        sideOffset={sideOffset}
        className={cn(
          "z-50 w-72 rounded-lg bg-popover p-4 text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
          className
        )}
        data-slot="popover-content"
        {...props}
      />
    </PopoverPrimitive.Portal>
  )
}

export { Popover, PopoverTrigger, PopoverContent, PopoverAnchor }