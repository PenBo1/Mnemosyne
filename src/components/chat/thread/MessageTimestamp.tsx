import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import { formatMessageTimestamp } from "./DateSeparator";
import type { MessageTimestampProps } from "@/types/chat";

export function MessageTimestamp({
  timestamp,
  format = "smart",
  className,
}: MessageTimestampProps) {
  const { t } = useI18n();

  const formatted =
    format === "smart"
      ? formatMessageTimestamp(timestamp, {
          today: t.agentChat.today,
          yesterday: t.agentChat.yesterday,
        })
      : format === "relative"
      ? getRelativeTime(timestamp)
      : new Date(timestamp).toLocaleTimeString(undefined, {
          hour: "numeric",
          minute: "2-digit",
        });

  return (
    <span
      className={cn(
        "text-xs text-muted-foreground/60 shrink-0",
        className
      )}
      title={new Date(timestamp).toLocaleString()}
    >
      {formatted}
    </span>
  );
}

// Get relative time (e.g. "2 min ago")
function getRelativeTime(timestamp: string | Date | number): string {
  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);

  if (diffSec < 60) return "刚刚";
  if (diffMin < 60) return `${diffMin} 分钟前`;
  if (diffHour < 24) return `${diffHour} 小时前`;

  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}