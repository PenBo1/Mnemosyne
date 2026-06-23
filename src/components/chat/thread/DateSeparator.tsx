import { cn } from "@/lib/utils";

interface DateSeparatorProps {
  label: string;
  className?: string;
}

export function DateSeparator({ label, className }: DateSeparatorProps) {
  return (
    <div className={cn("flex items-center justify-center py-2 my-2", className)}>
      <div className="relative flex items-center justify-center w-full">
        <div className="absolute inset-0 flex items-center">
          <div className="w-full border-t border-border/50" />
        </div>
        <span className="relative bg-background px-3 text-xs text-muted-foreground">
          {label}
        </span>
      </div>
    </div>
  );
}

// Format date key for grouping (YYYY-MM-DD)
export function formatDateKey(timestamp: string | Date | number): string {
  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
  return date.toISOString().split("T")[0];
}

// Get start of day for comparison
function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

// Format timestamp for display with smart labels
export function formatMessageTimestamp(
  timestamp: string | Date | number,
  labels: { today: string; yesterday: string }
): string {
  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
  const dayDelta = Math.round(
    (startOfDay(new Date()) - startOfDay(date)) / 86_400_000
  );

  const timeStr = date.toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });

  if (dayDelta === 0) {
    return `${labels.today} ${timeStr}`;
  }
  if (dayDelta === 1) {
    return `${labels.yesterday} ${timeStr}`;
  }

  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

// Get date label for separator
export function getDateLabel(
  dateKey: string,
  labels: { today: string; yesterday: string }
): string {
  const date = new Date(dateKey + "T00:00:00");
  const dayDelta = Math.round(
    (startOfDay(new Date()) - startOfDay(date)) / 86_400_000
  );

  if (dayDelta === 0) return labels.today;
  if (dayDelta === 1) return labels.yesterday;

  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}