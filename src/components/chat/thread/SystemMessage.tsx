import { cn } from "@/lib/utils";
import { MessageTimestamp } from "./MessageTimestamp";
import type { MessageItemProps } from "@/types/chat";

export function SystemMessage({
  message,
  className,
}: MessageItemProps) {
  return (
    <div className={cn("flex justify-center w-full", className)}>
      <div
        className={cn(
          "max-w-[90%] rounded-lg px-3 py-1.5 text-xs",
          "bg-muted/30 text-muted-foreground border border-border/20"
        )}
        data-role="system"
      >
        <p className="whitespace-pre-wrap">{message.content}</p>
        <MessageTimestamp timestamp={message.created_at} className="mt-1" />
      </div>
    </div>
  );
}