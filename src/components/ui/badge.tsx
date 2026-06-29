import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Slot } from "radix-ui"

import { cn } from "@/shared/utils"

const badgeVariants = cva(
  "group/badge inline-flex h-[18px] w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-[var(--radius-4)] border border-transparent px-2 py-0.5 text-[11px] font-medium whitespace-nowrap transition-all focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&>svg]:pointer-events-none [&>svg]:size-2.5!",
  {
    variants: {
      variant: {
        default: "bg-[var(--bg-brand)] text-[var(--text-onbrand)] [a]:hover:bg-[var(--bg-brand-hover)]",
        secondary:
          "bg-secondary text-secondary-foreground [a]:hover:bg-secondary/80",
        destructive:
          "bg-[var(--status-error-surface-l1)] text-[var(--status-error-default)] focus-visible:ring-[var(--status-error-surface-l1)] [a]:hover:bg-[var(--status-error-surface-l2)]",
        outline:
          "border-[var(--border-neutral-l2)] bg-[var(--bg-overlay-l1)] text-[var(--text-secondary)] [a]:hover:bg-[var(--bg-overlay-l2)] [a]:hover:text-[var(--text-default)]",
        success: "bg-[var(--status-success-surface-l1)] text-[var(--status-success-default)]",
        warning: "bg-[var(--status-warning-surface-l1)] text-[var(--status-warning-default)]",
        info: "bg-[var(--status-primary-surface-l1)] text-[var(--status-primary-default)]",
        ghost:
          "hover:bg-muted hover:text-muted-foreground dark:hover:bg-muted/50",
        link: "text-primary underline-offset-4 hover:underline",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

function Badge({
  className,
  variant = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot.Root : "span"

  return (
    <Comp
      data-slot="badge"
      data-variant={variant}
      className={cn(badgeVariants({ variant }), className)}
      {...props}
    />
  )
}

export { Badge, badgeVariants }
