/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Spinner - 加载旋转组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { cn } from "@/lib/utils"
import { Loader2Icon } from "lucide-react"

// ── 加载旋转组件 ────────────────────────────────────────────────────────────

function Spinner({ className, ...props }: React.ComponentProps<"svg">) {
  return (
    <Loader2Icon role="status" aria-label="Loading" className={cn("size-4 animate-spin", className)} {...props} />
  )
}

export { Spinner }
