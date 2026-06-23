import { cn } from "@/lib/utils";
import { MarkdownRenderer } from "../markdown/MarkdownRenderer";
import { MessageTimestamp } from "./MessageTimestamp";
import { CopyIcon } from "lucide-react";
import { useCallback } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/lib/i18n";
import type { MessageItemProps } from "@/types/chat";

export function AssistantMessage({
  message,
  onCopy,
  className,
}: MessageItemProps) {
  const { t } = useI18n();

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(message.content);
    onCopy?.();
  }, [message.content, onCopy]);

  return (
    <div className={cn("flex justify-start w-full", className)}>
      <div
        className={cn(
          "max-w-[85%] rounded-lg px-3 py-2 text-sm",
          "bg-muted/50 border border-border/30"
        )}
        data-role="assistant"
      >
        <MarkdownRenderer content={message.content} />
        <div className="flex items-center justify-end gap-2 mt-1.5">
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={handleCopy}
            aria-label={t.agentChat.copyMessage}
            className="opacity-60 hover:opacity-100"
          >
            <CopyIcon className="size-3" />
          </Button>
          <MessageTimestamp timestamp={message.created_at} />
        </div>
      </div>
    </div>
  );
}