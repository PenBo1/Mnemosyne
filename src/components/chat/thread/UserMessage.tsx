import { cn } from "@/lib/utils";
import { MessageTimestamp } from "./MessageTimestamp";
import type { MessageItemProps } from "@/types/chat";

export function UserMessage({ message, className }: MessageItemProps) {
  return (
    <div className={cn("flex justify-end w-full", className)}>
      <div
        className={cn(
          "max-w-[85%] rounded-lg px-3 py-2 text-sm",
          "bg-primary text-primary-foreground"
        )}
        data-role="user"
      >
        <p className="whitespace-pre-wrap">{message.content}</p>
        <div className="flex items-center justify-end gap-2 mt-1.5">
          <MessageTimestamp timestamp={message.created_at} />
        </div>
      </div>
    </div>
  );
}