/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Table - 表格组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"

import { cn } from "@/lib/utils"

// ── 表格组件 ────────────────────────────────────────────────────────────────

function Table({ className, ...props }: React.ComponentProps<"table">) {
  return (
    <div
      data-slot="table-container"
      className="relative w-full overflow-x-auto"
    >
      <table
        data-slot="table"
        className={cn("w-full caption-bottom text-xs", className)}
        {...props}
      />
    </div>
  )
}

// ── 表格头部组件 ────────────────────────────────────────────────────────────

function TableHeader({ className, ...props }: React.ComponentProps<"thead">) {
  return (
    <thead
      data-slot="table-header"
      className={cn("[&_tr]:border-b", className)}
      {...props}
    />
  )
}

// ── 表格主体组件 ────────────────────────────────────────────────────────────

function TableBody({ className, ...props }: React.ComponentProps<"tbody">) {
  return (
    <tbody
      data-slot="table-body"
      className={cn("[&_tr:last-child]:border-0", className)}
      {...props}
    />
  )
}

// ── 表格底部组件 ────────────────────────────────────────────────────────────

function TableFooter({ className, ...props }: React.ComponentProps<"tfoot">) {
  return (
    <tfoot
      data-slot="table-footer"
      className={cn(
        "border-t bg-[var(--bg-overlay-l2)] font-medium [&>tr]:last:border-b-0",
        className
      )}
      {...props}
    />
  )
}

// ── 表格行组件 ──────────────────────────────────────────────────────────────

function TableRow({ className, ...props }: React.ComponentProps<"tr">) {
  return (
    <tr
      data-slot="table-row"
      className={cn(
        "border-b transition-colors hover:bg-[var(--bg-overlay-l2)] has-aria-expanded:bg-[var(--bg-overlay-l2)] data-[state=selected]:bg-[var(--bg-overlay-l2)]",
        className
      )}
      {...props}
    />
  )
}

// ── 表格头单元格组件 ────────────────────────────────────────────────────────

function TableHead({ className, ...props }: React.ComponentProps<"th">) {
  return (
    <th
      data-slot="table-head"
      className={cn(
        "h-10 px-2 text-left align-middle font-medium whitespace-nowrap text-[var(--text-default)] [&:has([role=checkbox])]:pr-0",
        className
      )}
      {...props}
    />
  )
}

// ── 表格单元格组件 ──────────────────────────────────────────────────────────

function TableCell({ className, ...props }: React.ComponentProps<"td">) {
  return (
    <td
      data-slot="table-cell"
      className={cn(
        "p-2 align-middle whitespace-nowrap [&:has([role=checkbox])]:pr-0",
        className
      )}
      {...props}
    />
  )
}

// ── 表格标题组件 ────────────────────────────────────────────────────────────

function TableCaption({
  className,
  ...props
}: React.ComponentProps<"caption">) {
  return (
    <caption
      data-slot="table-caption"
      className={cn("mt-4 text-xs text-muted-foreground", className)}
      {...props}
    />
  )
}

export {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
  TableHead,
  TableRow,
  TableCell,
  TableCaption,
}
