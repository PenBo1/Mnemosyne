import { useMemo } from "react";
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  MessageScrollerProvider,
  MessageScroller,
  MessageScrollerViewport,
  MessageScrollerContent,
  MessageScrollerItem,
  MessageScrollerButton,
} from "@/components/ui/message-scroller";
import { MessageGroup } from "@/components/ui/message";
import { useAgentStore } from "@/features/chat/store";
import { MessageBubble } from "./message-bubble";
import { EmptyState } from "./empty-state";
import type { Message as ChatMessage } from "@/types";

interface MessageListProps {
  messages: ChatMessage[];
  streaming: boolean;
  error: string | null;
  onRegenerate: () => void;
}

/** 消息列表 —— 使用 MessageGroup 对连续同发送者消息分组 */
export function MessageList({ messages, streaming, error, onRegenerate }: MessageListProps) {
  const streamingContent = useAgentStore((s) => s.streamingContent);
  const streamingReasoning = useAgentStore((s) => s.streamingReasoning);
  const activeToolCalls = useAgentStore((s) => s.activeToolCalls);

  const isEmpty = messages.length === 0 && !streaming;

  const lastAssistantIdx = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "assistant") return i;
    }
    return -1;
  }, [messages]);

  // 将连续同角色消息分组
  const groups = useMemo(() => {
    const result: Array<{ role: string; messages: ChatMessage[] }> = [];
    for (const msg of messages) {
      const last = result[result.length - 1];
      if (last && last.role === msg.role) {
        last.messages.push(msg);
      } else {
        result.push({ role: msg.role, messages: [msg] });
      }
    }
    return result;
  }, [messages]);

  if (isEmpty) {
    return (
      <div className="flex-1 overflow-hidden">
        <EmptyState />
      </div>
    );
  }

  return (
    <MessageScrollerProvider>
      <MessageScroller className="flex-1 bg-background">
        <MessageScrollerViewport>
          <MessageScrollerContent className="mx-auto max-w-3xl px-4 py-6 gap-5">
            {groups.map((group, groupIdx) => (
              <MessageScrollerItem key={groupIdx}>
                <MessageGroup>
                  {group.messages.map((msg, msgIdx) => {
                    // 计算全局索引
                    const globalIdx = groups
                      .slice(0, groupIdx)
                      .reduce((acc, g) => acc + g.messages.length, 0) + msgIdx;
                    return (
                      <MessageBubble
                        key={msg.id}
                        message={msg}
                        onRegenerate={
                          !streaming && msg.role === "assistant" && globalIdx === lastAssistantIdx
                            ? onRegenerate
                            : undefined
                        }
                      />
                    );
                  })}
                </MessageGroup>
              </MessageScrollerItem>
            ))}

            {streaming && (
              <MessageScrollerItem scrollAnchor>
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
                  toolCalls={activeToolCalls}
                />
              </MessageScrollerItem>
            )}

            {error && (
              <MessageScrollerItem>
                <Alert variant="destructive">
                  <AlertCircle className="size-4" />
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              </MessageScrollerItem>
            )}
          </MessageScrollerContent>
        </MessageScrollerViewport>
        <MessageScrollerButton
          direction="end"
          className="border-border bg-background shadow-md hover:bg-muted"
        />
      </MessageScroller>
    </MessageScrollerProvider>
  );
}
