import * as React from "react"
import { cn } from "@/lib/utils"
import { ScrollArea } from "@/components/ui/scroll-area"

function PageContainer({
  className,
  scrollable = true,
  children,
  ...props
}: React.ComponentProps<"div"> & { scrollable?: boolean }) {
  if (scrollable) {
    return (
      <ScrollArea className={cn("h-full", className)}>
        <div
          data-slot="page-container"
          className="flex flex-col gap-6 p-6"
          {...props}
        >
          {children}
        </div>
      </ScrollArea>
    )
  }

  return (
    <div
      data-slot="page-container"
      className={cn("flex h-full flex-col gap-6 p-6", className)}
      {...props}
    >
      {children}
    </div>
  )
}

function PageHeader({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="page-header"
      className={cn("flex flex-wrap items-center justify-between gap-4", className)}
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
      className={cn("flex items-center gap-2 text-lg font-semibold tracking-tight", className)}
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