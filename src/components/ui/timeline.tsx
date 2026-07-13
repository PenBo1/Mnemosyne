import * as React from "react"
import { cn } from "@/lib/utils"

function Timeline({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline"
      className={cn("flex flex-col gap-4", className)}
      {...props}
    />
  )
}

function TimelineItem({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline-item"
      className={cn("flex items-start gap-4 relative", className)}
      {...props}
    />
  )
}

function TimelineSeparator({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline-separator"
      className={cn("flex flex-col items-center gap-1", className)}
      {...props}
    />
  )
}

function TimelineDot({
  className,
  variant = "default",
  ...props
}: React.ComponentProps<"div"> & {
  variant?: "default" | "primary" | "secondary" | "destructive" | "muted"
}) {
  const variantStyles = {
    default: "bg-primary border-primary",
    primary: "bg-primary border-primary",
    secondary: "bg-secondary border-secondary",
    destructive: "bg-destructive border-destructive",
    muted: "bg-muted-foreground border-muted-foreground",
  }

  return (
    <div
      data-slot="timeline-dot"
      className={cn(
        "size-3 rounded-full border-2 shrink-0 mt-1",
        variantStyles[variant],
        className
      )}
      {...props}
    />
  )
}

function TimelineConnector({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline-connector"
      className={cn(
        "w-px flex-1 bg-border min-h-4",
        className
      )}
      {...props}
    />
  )
}

function TimelineContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline-content"
      className={cn("flex flex-col gap-1 flex-1 min-w-0", className)}
      {...props}
    />
  )
}

function TimelineHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="timeline-header"
      className={cn("flex items-center gap-2", className)}
      {...props}
    />
  )
}

function TimelineTitle({ className, ...props }: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="timeline-title"
      className={cn("font-medium", className)}
      {...props}
    />
  )
}

function TimelineDescription({ className, ...props }: React.ComponentProps<"p">) {
  return (
    <p
      data-slot="timeline-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  )
}

function TimelineDate({ className, ...props }: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="timeline-date"
      className={cn("text-xs text-muted-foreground shrink-0", className)}
      {...props}
    />
  )
}

export {
  Timeline,
  TimelineItem,
  TimelineSeparator,
  TimelineDot,
  TimelineConnector,
  TimelineContent,
  TimelineHeader,
  TimelineTitle,
  TimelineDescription,
  TimelineDate,
}