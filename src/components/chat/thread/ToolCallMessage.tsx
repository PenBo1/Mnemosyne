import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import type { MessageItemProps } from "@/types/chat";

export function ToolCallMessage({
  message,
  className,
}: MessageItemProps) {
  const { t } = useI18n();

  const toolCalls = message.tool_calls
    ? (() => {
        try {
          return JSON.parse(message.tool_calls);
        } catch {
          return null;
        }
      })()
    : null;

  return (
    <div className={cn("flex justify-start w-full", className)}>
      <div
        className={cn(
          "max-w-[85%] rounded-lg px-3 py-2 text-xs",
          "bg-muted/30 border border-border/20 font-mono"
        )}
        data-role="tool"
      >
        {toolCalls ? (
          <div className="opacity-70">
            <span className="font-medium">{t.agentChat.toolCalls}</span>
            {toolCalls.map((tc: { name: string }, index: number) => (
              <span key={index} className="ml-1 text-muted-foreground">
                {tc.name}
              </span>
            ))}
          </div>
        ) : (
          <div className="opacity-70">
            <span className="font-medium">{t.agentChat.toolResult}</span>
            {message.content.slice(0, 150)}
            {message.content.length > 150 && "..."}
          </div>
        )}
      </div>
    </div>
  );
}