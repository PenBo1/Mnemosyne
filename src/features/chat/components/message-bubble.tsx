/**
 * ═══════════════════════════════════════════════════════════════════════════
 * MessageBubble - 聊天消息气泡组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo, useState, Suspense, lazy, useMemo } from "react";
import {
  Copy,
  Check,
  RotateCcw,
  ChevronDown,
  Wrench,
  Sparkles,
  CheckCircle2,
  AlertCircle,
  Brain,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Spinner } from "@/components/ui/spinner";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Marker, MarkerIcon, MarkerContent } from "@/components/ui/marker";
import { Bubble, BubbleContent } from "@/components/ui/bubble";
import { Message, MessageContent, MessageFooter } from "@/components/ui/message";
import { useCopy } from "@/features/chat/hooks/use-copy";
import { ThinkingIndicator } from "@/features/agent/components/ThinkingIndicator";
import type { ActiveToolCall } from "@/features/chat/store";
import type { Message as ChatMessage } from "@/types";

// ── 懒加载组件 ──────────────────────────────────────────────────────────────

const MarkdownRenderer = lazy(() =>
  import("./markdown-renderer").then((m) => ({ default: m.MarkdownRenderer })),
);

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 格式化时间为本地时间字符串
 */
function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 流式消息光标
 */
function StreamingCursor() {
  return (
    <span
      className="ml-0.5 inline-block h-3.5 w-[2px] translate-y-0.5 animate-pulse rounded-full bg-primary"
      aria-hidden="true"
    />
  );
}

/**
 * 助手头像
 */
function AssistantAvatar() {
  return (
    <div className="flex size-7 shrink-0 items-center justify-center self-start rounded-full bg-[var(--bg-brand-popup)] text-[var(--text-brand)]">
      <Sparkles className="size-3.5" />
    </div>
  );
}

/**
 * 推理过程展示区块
 */
function ReasoningBlock({
  reasoning,
  isStreaming,
}: {
  reasoning: string;
  isStreaming: boolean;
}) {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);

  if (!reasoning && !isStreaming) return null;

  const open = expanded || isStreaming;

  return (
    <Collapsible
      open={open}
      onOpenChange={setExpanded}
      className="mb-2 overflow-hidden rounded-lg border border-border/60 bg-muted/40"
    >
      <CollapsibleTrigger className="flex w-full items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground">
        {isStreaming ? (
          <>
            <Spinner className="size-3" />
            <span>{t.agentChat.thinking}</span>
          </>
        ) : (
          <>
            <Brain className="size-3" />
            <span className="uppercase tracking-wide">{t.agentChat.reasoningTitle}</span>
            <ChevronDown
              className={cn("ml-auto size-3 transition-transform", expanded && "rotate-180")}
            />
          </>
        )}
      </CollapsibleTrigger>
      {reasoning && (
        <CollapsibleContent>
          <div className="border-t border-border/60 px-3 py-2">
            <p className="whitespace-pre-wrap text-xs leading-relaxed text-muted-foreground">
              {reasoning}
            </p>
          </div>
        </CollapsibleContent>
      )}
    </Collapsible>
  );
}

/**
 * 工具调用卡片
 */
function ToolCallCard({ call }: { call: ActiveToolCall }) {
  const { t } = useI18n();
  const isRunning = call.status === "running";
  return (
    <Marker variant="border" className="my-0.5 gap-2 py-1.5 max-w-full">
      <MarkerIcon className="size-3.5">
        {isRunning ? (
          <Spinner className="size-3 text-muted-foreground" />
        ) : call.status === "error" ? (
          <AlertCircle className="size-3 text-destructive" />
        ) : (
          <CheckCircle2 className="size-3 text-[var(--text-brand)]" />
        )}
      </MarkerIcon>
      <MarkerContent className="flex items-baseline gap-1 min-w-0">
        <span className="font-mono text-[11px] text-muted-foreground shrink-0">
          {isRunning ? t.agentChat.toolRunning : t.agentChat.toolCalled}
        </span>
        <span className="font-mono text-[11px] font-medium text-foreground truncate">
          {call.name}
        </span>
      </MarkerContent>
    </Marker>
  );
}

/**
 * 工具结果展示
 */
function ToolResult({ content }: { content: string }) {
  const { t } = useI18n();
  return (
    <Marker variant="border" className="my-1.5">
      <MarkerIcon>
        <Wrench className="text-muted-foreground" />
      </MarkerIcon>
      <MarkerContent>
        <p className="mb-0.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          {t.agentChat.toolResult}
        </p>
        <pre className="whitespace-pre-wrap break-all font-mono text-xs text-foreground">
          {content}
        </pre>
      </MarkerContent>
    </Marker>
  );
}

/**
 * 文本块组件
 */
function TextBlock({
  content,
  isStreaming,
  showCursor,
}: {
  content: string;
  isStreaming?: boolean;
  showCursor?: boolean;
}) {
  const { t } = useI18n();
  if (!content) return null;

  if (isStreaming) {
    return (
      <p className="whitespace-pre-wrap break-words leading-relaxed">
        {content}
        {showCursor && <StreamingCursor />}
      </p>
    );
  }

  return (
    <Suspense
      fallback={<span className="text-muted-foreground animate-pulse">{t.chat.message.loading}</span>}
    >
      <MarkdownRenderer content={content} />
    </Suspense>
  );
}

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface RenderSegment {
  type: "text" | "tool";
  content: string;
  toolCall?: ActiveToolCall;
  position: number;
}

interface MessageBubbleProps {
  message: ChatMessage;
  isStreaming?: boolean;
  reasoning?: string;
  toolCalls?: ActiveToolCall[];
  onRegenerate?: () => void;
}

/**
 * 构建渲染片段，将文本和工具调用分离
 */
function buildRenderSegments(
  content: string,
  toolCalls: ActiveToolCall[] | undefined,
  isStreaming?: boolean
): RenderSegment[] {
  if (!toolCalls || toolCalls.length === 0) {
    if (!content) return [];
    return [{ type: "text", content, position: 0 }];
  }

  const sortedCalls = [...toolCalls].sort((a, b) => a.startPosition - b.startPosition);

  const segments: RenderSegment[] = [];
  let lastEnd = 0;

  for (const call of sortedCalls) {
    const textBefore = content.slice(lastEnd, call.startPosition);
    if (textBefore) {
      segments.push({ type: "text", content: textBefore, position: lastEnd });
    }

    const toolEnd = call.endPosition ?? (isStreaming ? content.length : call.startPosition);
    segments.push({
      type: "tool",
      content: "",
      toolCall: call,
      position: call.startPosition,
    });

    lastEnd = Math.max(lastEnd, toolEnd);
  }

  const textAfter = content.slice(lastEnd);
  if (textAfter) {
    segments.push({ type: "text", content: textAfter, position: lastEnd });
  }

  return segments;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 消息气泡组件，展示单条聊天消息
 */
export const MessageBubble = memo(function MessageBubble({
  message,
  isStreaming,
  reasoning,
  toolCalls,
  onRegenerate,
}: MessageBubbleProps) {
  const { t } = useI18n();
  const { copied, copy } = useCopy();
  const isUser = message.role === "user";

  const segments = useMemo(
    () => buildRenderSegments(message.content, toolCalls, isStreaming),
    [message.content, toolCalls, isStreaming]
  );

  if (message.role === "system") {
    return (
      <Marker variant="separator">
        <MarkerContent>{message.content}</MarkerContent>
      </Marker>
    );
  }

  if (message.role === "tool") {
    return <ToolResult content={message.content} />;
  }

  const hasContent = message.content.length > 0;
  const hasReasoning = reasoning && reasoning.length > 0;
  const showReasoning = hasReasoning || (isStreaming && !hasContent);

  if (isUser) {
    return (
      <Message align="end">
        <MessageContent>
          <Bubble variant="secondary" align="end">
            <BubbleContent>
              <p className="whitespace-pre-wrap break-words leading-relaxed">{message.content}</p>
            </BubbleContent>
          </Bubble>
          <MessageFooter className="justify-end">
            <span className="text-[10px] text-muted-foreground opacity-0 transition-opacity group-hover/message:opacity-100">
              {formatTime(message.created_at)}
            </span>
          </MessageFooter>
        </MessageContent>
      </Message>
    );
  }

  return (
    <Message align="start">
      <AssistantAvatar />
      <MessageContent className="max-w-[85%]">
        {showReasoning && (
          <ReasoningBlock reasoning={reasoning ?? ""} isStreaming={!!isStreaming} />
        )}

        {segments.length > 0 ? (
          <div className="flex flex-col gap-1">
            {segments.map((segment, idx) => {
              if (segment.type === "text") {
                const isLast = idx === segments.length - 1;
                return (
                  <Bubble key={`text-${idx}`} variant="ghost" align="start">
                    <BubbleContent>
                      <TextBlock
                        content={segment.content}
                        isStreaming={isStreaming && isLast}
                        showCursor={isStreaming && isLast}
                      />
                    </BubbleContent>
                  </Bubble>
                );
              } else if (segment.toolCall) {
                return (
                  <ToolCallCard
                    key={`tool-${segment.toolCall.id}`}
                    call={segment.toolCall}
                  />
                );
              }
              return null;
            })}
          </div>
        ) : isStreaming ? (
          <Bubble variant="ghost" align="start">
            <BubbleContent>
              <div className="flex items-center gap-2">
                <ThinkingIndicator effect="shade-fire" />
                <span className="text-sm text-muted-foreground">{t.agentChat.thinking}</span>
              </div>
            </BubbleContent>
          </Bubble>
        ) : null}

        {!isStreaming && hasContent && (
          <MessageFooter>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  onClick={() => {
                    void copy(message.content);
                  }}
                  aria-label={t.agentChat.copyMessage}
                  className="size-6 text-muted-foreground hover:text-foreground"
                >
                  {copied ? (
                    <Check className="size-3 text-[var(--text-brand)]" />
                  ) : (
                    <Copy className="size-3" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">{t.agentChat.copyMessage}</TooltipContent>
            </Tooltip>
            {onRegenerate && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    onClick={onRegenerate}
                    aria-label={t.agentChat.regenerate}
                    className="size-6 text-muted-foreground hover:text-foreground"
                  >
                    <RotateCcw className="size-3" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="top">{t.agentChat.regenerate}</TooltipContent>
              </Tooltip>
            )}
            <span className="ml-1 text-[10px] text-muted-foreground">
              {formatTime(message.created_at)}
            </span>
          </MessageFooter>
        )}
      </MessageContent>
    </Message>
  );
});