import { BotIcon, Check, Copy, UserIcon } from "lucide-react";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { useI18n } from "@/shared/i18n";
import { useCopyFeedback } from "@/hooks/useCopyFeedback";
import type { Message } from "@/shared/types";

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function MessageBubble({
  message,
  isStreaming,
}: {
  message: Message;
  isStreaming?: boolean;
}) {
  const { t } = useI18n();
  const { copied, copy } = useCopyFeedback();

  const handleCopy = () => copy(message.content);

  // user: 右对齐，primary 色气泡
  if (message.role === "user") {
    return (
      <div className="flex justify-end" data-user-message-id={message.id}>
        <div className="flex max-w-[75%] flex-row-reverse items-start gap-2.5">
          <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-primary/10">
            <UserIcon className="size-3.5 text-primary" />
          </div>
          <div
            className="rounded-2xl rounded-tr-md px-4 py-2.5 text-sm leading-[1.6]"
            style={{
              background: "var(--color-primary)",
              color: "var(--color-text-on-primary)",
            }}
          >
            <p className="whitespace-pre-wrap">{message.content}</p>
          </div>
        </div>
      </div>
    );
  }

  // system: 居中小标签
  if (message.role === "system") {
    return (
      <div className="flex justify-center">
        <div className="rounded-full bg-muted/50 px-3 py-1 text-[11px] text-muted-foreground">
          {message.content}
        </div>
      </div>
    );
  }

  // tool: 折叠卡片
  if (message.role === "tool") {
    return (
      <div className="flex justify-start">
        <div className="flex flex-col gap-1 max-w-[80%] rounded-lg border border-border bg-muted/30 px-3 py-2">
          <p className="text-[11px] font-medium text-muted-foreground">
            {t.agentChat.toolResult}
          </p>
          <pre className="whitespace-pre-wrap text-xs text-foreground/80">
            {message.content}
          </pre>
        </div>
      </div>
    );
  }

  // assistant: 左对齐，card 背景，markdown 渲染
  const showCursor = isStreaming;
  const showThinking = isStreaming && !message.content;

  return (
    <div className="flex justify-start">
      <div className="flex max-w-[85%] items-start gap-2.5">
        <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-primary/10">
          <BotIcon className="size-3.5 text-primary" />
        </div>
        <div className="flex flex-col gap-1">
          <div className="rounded-2xl rounded-tl-md border border-border bg-card px-4 py-3 shadow-sm">
            {showThinking ? (
              <span className="inline-flex items-center gap-1">
                <span className="size-1.5 animate-pulse rounded-full bg-primary" />
                <span className="size-1.5 animate-pulse rounded-full bg-primary [animation-delay:150ms]" />
                <span className="size-1.5 animate-pulse rounded-full bg-primary [animation-delay:300ms]" />
              </span>
            ) : (
              <>
                <MarkdownRenderer content={message.content} />
                {showCursor && (
                  <span className="ml-0.5 inline-block h-3.5 w-[3px] animate-pulse bg-primary align-text-bottom" />
                )}
              </>
            )}
          </div>
          {!isStreaming && (
            <div className="flex items-center gap-2 px-1">
              <span className="text-[10px] text-muted-foreground/50">
                {formatTime(message.created_at)}
              </span>
              <button
                type="button"
                onClick={handleCopy}
                className="flex items-center gap-1 rounded px-1 py-0.5 text-[10px] text-muted-foreground/50 transition-colors hover:bg-muted hover:text-muted-foreground"
              >
                {copied ? <Check className="size-2.5" /> : <Copy className="size-2.5" />}
                {copied ? t.agentChat.copied : t.agentChat.copyMessage}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
