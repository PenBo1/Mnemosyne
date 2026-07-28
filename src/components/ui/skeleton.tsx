/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Skeleton - 骨架屏组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { cn } from "@/lib/utils"

// ── 骨架屏组件 ──────────────────────────────────────────────────────────────

function Skeleton({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="skeleton"
      className={cn("animate-pulse rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]", className)}
      {...props}
    />
  )
}

export { Skeleton }
