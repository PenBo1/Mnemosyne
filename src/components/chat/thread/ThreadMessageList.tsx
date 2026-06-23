import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import { Spinner } from "@/components/ui/spinner";
import { AssistantMessage } from "./AssistantMessage";
import { UserMessage } from "./UserMessage";
import { SystemMessage } from "./SystemMessage";
import { ToolCallMessage } from "./ToolCallMessage";
import { DateSeparator, formatDateKey, getDateLabel } from "./DateSeparator";
import type { ChatMessage } from "@/types/chat";
import type { ThreadMessageListProps } from "@/types/chat";

export function ThreadMessageList({
  messages,
  streamingContent,
  isStreaming,
  className,
}: ThreadMessageListProps) {
  const { t } = useI18n();

  // Group messages by date
  const messageGroups = useMemo(() => {
    const groups: Map<string, ChatMessage[]> = new Map();

    for (const msg of messages) {
      const dateKey = formatDateKey(msg.created_at);
      const existing = groups.get(dateKey) || [];
      existing.push(msg);
      groups.set(dateKey, existing);
    }

    return Array.from(groups.entries()).map(([date, msgs]) => ({
      date,
      label: getDateLabel(date, {
        today: t.agentChat.today,
        yesterday: t.agentChat.yesterday,
      }),
      messages: msgs,
    }));
  }, [messages, t.agentChat.today, t.agentChat.yesterday]);

  // Render single message based on role
  const renderMessage = (message: ChatMessage) => {
    switch (message.role) {
      case "user":
        return <UserMessage key={message.id} message={message} />;
      case "assistant":
        return <AssistantMessage key={message.id} message={message} />;
      case "system":
        return <SystemMessage key={message.id} message={message} />;
      case "tool":
        return <ToolCallMessage key={message.id} message={message} />;
      default:
        return null;
    }
  };

  return (
    <div className={cn("flex flex-col gap-3 overflow-y-auto py-3", className)}>
      {messageGroups.map((group) => (
        <div key={group.date}>
          <DateSeparator label={group.label} />
          <div className="flex flex-col gap-2">
            {group.messages.map(renderMessage)}
          </div>
        </div>
      ))}

      {/* Streaming response */}
      {isStreaming && streamingContent && (
        <div className="flex justify-start animate-fade-in">
          <div
            className={cn(
              "max-w-[85%] rounded-lg px-3 py-2 text-sm",
              "bg-muted/50 border border-border/30"
            )}
            data-role="assistant"
            data-streaming="true"
          >
            <p className="whitespace-pre-wrap">{streamingContent}</p>
            <span className="inline-block w-2 h-4 bg-foreground animate-pulse ml-1" />
          </div>
        </div>
      )}

      {/* Loading indicator when streaming but no content yet */}
      {isStreaming && !streamingContent && (
        <div className="flex justify-start animate-fade-in">
          <div
            className={cn(
              "max-w-[85%] rounded-lg px-3 py-2 text-sm",
              "bg-muted/50 border border-border/30"
            )}
          >
            <div className="flex items-center gap-2 text-muted-foreground">
              <Spinner className="size-3" />
              <span>{t.agentChat.thinking}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}