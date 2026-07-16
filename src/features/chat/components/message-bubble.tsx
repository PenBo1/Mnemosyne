import { memo, useState, Suspense, lazy } from "react";
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
import type { ActiveToolCall } from "@/features/chat/store";
import type { Message as ChatMessage } from "@/types";

const MarkdownRenderer = lazy(() =>
  import("./markdown-renderer").then((m) => ({ default: m.MarkdownRenderer })),
);

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** 流式光标 —— 闪烁竖条，比小圆点优雅 */
function StreamingCursor() {
  return (
    <span
      className="ml-0.5 inline-block h-3.5 w-[2px] translate-y-0.5 animate-pulse rounded-full bg-primary"
      aria-hidden="true"
    />
  );
}

/** AI 头像 —— Sparkles 图标 + primary 淡色背景 */
function AssistantAvatar() {
  return (
    <div className="flex size-7 shrink-0 items-center justify-center self-start rounded-full bg-[var(--bg-brand-popup)] text-[var(--text-brand)]">
      <Sparkles className="size-3.5" />
    </div>
  );
}

/** 思考过程折叠块 —— Collapsible + Brain 图标 + 淡色背景 */
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

  // 流式中强制展开，结束后用户可手动折叠/展开
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

/** 工具调用卡片 —— 进行中 Spinner / 完成 CheckCircle / 错误 AlertCircle */
function ToolCallCard({ call }: { call: ActiveToolCall }) {
  const { t } = useI18n();
  const isRunning = call.status === "running";
  return (
    <Marker variant="border" className="my-0.5 gap-2 py-1.5">
      <MarkerIcon className="size-3.5">
        {isRunning ? (
          <Spinner className="size-3 text-muted-foreground" />
        ) : call.status === "error" ? (
          <AlertCircle className="size-3 text-destructive" />
        ) : (
          <CheckCircle2 className="size-3 text-[var(--text-brand)]" />
        )}
      </MarkerIcon>
      <MarkerContent className="flex items-baseline gap-1">
        <span className="font-mono text-[11px] text-muted-foreground">
          {isRunning ? t.agentChat.toolRunning : t.agentChat.toolCalled}
        </span>
        <span className="font-mono text-[11px] font-medium text-foreground">
          {call.name}
        </span>
      </MarkerContent>
    </Marker>
  );
}

/** 工具结果卡片 —— 历史消息中的 tool 角色展示 */
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

interface MessageBubbleProps {
  message: ChatMessage;
  isStreaming?: boolean;
  reasoning?: string;
  /** 流式 turn 进行中/已完成的工具调用（仅流式 bubble 传入） */
  toolCalls?: ActiveToolCall[];
  onRegenerate?: () => void;
}

/** 消息气泡 —— shadcn Message + Bubble + Collapsible + Marker 组合 */
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

  // system 消息：使用 Marker separator
  if (message.role === "system") {
    return (
      <Marker variant="separator">
        <MarkerContent>{message.content}</MarkerContent>
      </Marker>
    );
  }

  // tool 消息：工具结果卡片
  if (message.role === "tool") {
    return <ToolResult content={message.content} />;
  }

  const hasContent = message.content.length > 0;
  const hasReasoning = reasoning && reasoning.length > 0;
  const showReasoning = hasReasoning || (isStreaming && !hasContent);
  const hasToolCalls = toolCalls && toolCalls.length > 0;

  // 用户消息：右对齐 secondary 气泡（比 primary 更柔和）
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

  // AI 消息：头像 + 思考 + 工具调用 + ghost 正文
  return (
    <Message align="start">
      <AssistantAvatar />
      <MessageContent>
        {showReasoning && (
          <ReasoningBlock reasoning={reasoning ?? ""} isStreaming={!!isStreaming} />
        )}

        {hasToolCalls && (
          <div className="flex flex-col">
            {toolCalls!.map((call) => (
              <ToolCallCard key={call.id} call={call} />
            ))}
          </div>
        )}

        {(hasContent || isStreaming) && (
          <Bubble variant="ghost" align="start">
            <BubbleContent>
              {hasContent ? (
                isStreaming ? (
                  // 流式期间用纯文本渲染 —— 避免 MarkdownRenderer 每帧全文 re-parse O(n²)
                  <p className="whitespace-pre-wrap break-words leading-relaxed">
                    {message.content}
                    <StreamingCursor />
                  </p>
                ) : (
                  <Suspense
                    fallback={
                      <span className="text-muted-foreground animate-pulse">Loading...</span>
                    }
                  >
                    <MarkdownRenderer content={message.content} />
                  </Suspense>
                )
              ) : (
                isStreaming && <StreamingCursor />
              )}
            </BubbleContent>
          </Bubble>
        )}

        {/* 操作栏：悬停显示 */}
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
