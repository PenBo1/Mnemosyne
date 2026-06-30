import { useEffect, type RefObject } from "react";
import { AlertCircle } from "lucide-react";
import { MessageBubble } from "./MessageBubble";
import { EmptyState } from "./EmptyState";
import type { Message } from "@/shared/types";

export function MessageList({
  messages,
  streaming,
  streamingContent,
  streamingReasoning,
  error,
  scrollRef,
}: {
  messages: Message[];
  streaming: boolean;
  streamingContent: string;
  streamingReasoning: string;
  error: string | null;
  scrollRef: RefObject<HTMLDivElement | null>;
}) {
  // Scroll to bottom on new messages
  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;
    container.scrollTop = container.scrollHeight;
  }, [messages.length, scrollRef]);

  // Follow bottom during streaming if user is near bottom
  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;
    const distance =
      container.scrollHeight - container.scrollTop - container.clientHeight;
    if (distance < 120) {
      container.scrollTop = container.scrollHeight;
    }
  }, [streamingContent, streamingReasoning, scrollRef]);

  const isEmpty = messages.length === 0 && !streaming;

  return (
    <div ref={scrollRef} className="flex-1 overflow-y-auto md-scrollbar">
      <div className="mx-auto flex min-h-full max-w-3xl flex-col gap-4 px-6 py-6">
        {isEmpty ? (
          <EmptyState />
        ) : (
          <>
            {messages.map((msg) => (
              <MessageBubble key={msg.id} message={msg} />
            ))}
            {streaming && (
              <MessageBubble
                message={{
                  id: "streaming",
                  session_id: "",
                  role: "assistant",
                  content: streamingContent,
                  tool_calls: null,
                  tool_results: null,
                  token_count: null,
                  created_at: new Date().toISOString(),
                }}
                isStreaming
                reasoning={streamingReasoning}
              />
            )}
            {error && (
              <div className="flex items-center gap-2 rounded-[var(--radius-6)] border border-[var(--status-error-default)]/20 bg-[var(--status-error-surface-l1)] px-3 py-2 text-xs text-[var(--status-error-default)]">
                <AlertCircle className="size-3.5 shrink-0" />
                <span>{error}</span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
