/**
 * ═══════════════════════════════════════════════════════════════════════════
 * MessageList - 聊天消息列表组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo, useMemo, useEffect } from "react";
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  MessageScrollerProvider,
  MessageScroller,
  MessageScrollerViewport,
  MessageScrollerContent,
  MessageScrollerItem,
  MessageScrollerButton,
  useMessageScroller,
} from "@/components/ui/message-scroller";
import { MessageGroup } from "@/components/ui/message";
import { useAgentStore } from "@/features/chat/store";
import { MessageBubble } from "./message-bubble";
import { EmptyState } from "./empty-state";
import type { Message as ChatMessage } from "@/types";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/**
 * 流式消息的固定时间戳，避免每帧生成新值
 */
const STREAMING_CREATED_AT = new Date(0).toISOString();

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface MessageListProps {
  messages: ChatMessage[];
  streaming: boolean;
  error: string | null;
  onRegenerate: () => void;
}

// ── 时间线滚动监听组件 ──────────────────────────────────────────────────────

/**
 * 监听时间线滚动事件并滚动到对应消息
 */
function TimelineScrollListener({
  groups,
}: {
  groups: Array<{ role: string; messages: ChatMessage[]; startIdx: number }>;
}) {
  const scrollerApi = useMessageScroller();

  useEffect(() => {
    const handleTimelineScroll = (e: CustomEvent<{ messageIndex: number }>) => {
      const { messageIndex } = e.detail;

      // 找到包含该消息索引的分组
      for (const group of groups) {
        const endIdx = group.startIdx + group.messages.length;
        if (messageIndex >= group.startIdx && messageIndex < endIdx) {
          // 找到该消息的 ID
          const msgIdx = messageIndex - group.startIdx;
          const msg = group.messages[msgIdx];
          if (msg && scrollerApi) {
            // 使用 messageId 滚动到对应消息
            scrollerApi.scrollToMessage(msg.id);
          }
          break;
        }
      }
    };

    window.addEventListener("timeline-scroll-to", handleTimelineScroll as EventListener);
    return () => {
      window.removeEventListener("timeline-scroll-to", handleTimelineScroll as EventListener);
    };
  }, [groups, scrollerApi]);

  return null;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 消息列表组件，展示聊天消息并支持滚动和流式更新
 */
export const MessageList = memo(function MessageList({
  messages,
  streaming,
  error,
  onRegenerate,
}: MessageListProps) {
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

  // 将连续同角色消息分组，预计算 startIdx 消除 globalIdx O(n²)
  const groups = useMemo(() => {
    const result: Array<{ role: string; messages: ChatMessage[]; startIdx: number }> = [];
    let idx = 0;
    for (const msg of messages) {
      const last = result[result.length - 1];
      if (last && last.role === msg.role) {
        last.messages.push(msg);
      } else {
        result.push({ role: msg.role, messages: [msg], startIdx: idx });
      }
      idx++;
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
      <TimelineScrollListener groups={groups} />
      <MessageScroller className="flex-1 bg-background">
        <MessageScrollerViewport>
          <MessageScrollerContent className="mx-auto max-w-3xl px-4 py-6 gap-5">
            {groups.map((group, groupIdx) => (
              <MessageScrollerItem key={groupIdx}>
                <MessageGroup>
                  {group.messages.map((msg, msgIdx) => {
                    // O(1) 全局索引
                    const globalIdx = group.startIdx + msgIdx;
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
                    created_at: STREAMING_CREATED_AT,
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
});