import { cn } from "@/shared/utils"

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
