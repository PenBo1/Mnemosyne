import { Check, Copy, ThumbsDown, ThumbsUp, RotateCcw } from "lucide-react";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { ThinkingProcess } from "./ThinkingProcess";
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
  reasoning,
}: {
  message: Message;
  isStreaming?: boolean;
  reasoning?: string;
}) {
  const { t } = useI18n();
  const { copied, copy } = useCopyFeedback();

  const handleCopy = () => copy(message.content);

  // ── User message: gray bubble, no avatar, like TRAE Code ──
  if (message.role === "user") {
    return (
      <div
        className="flex justify-start"
        data-user-message-id={message.id}
      >
        <div className="max-w-[85%] rounded-[var(--radius-8)] bg-[var(--bg-base-secondary)] px-4 py-3 text-sm leading-[1.7] text-[var(--text-default)]">
          <p className="whitespace-pre-wrap">{message.content}</p>
        </div>
      </div>
    );
  }

  // ── System message: centered pill ──
  if (message.role === "system") {
    return (
      <div className="flex justify-center">
        <div className="rounded-full bg-[var(--bg-overlay-l1)] px-3 py-1 text-[11px] text-[var(--text-tertiary)]">
          {message.content}
        </div>
      </div>
    );
  }

  // ── Tool message: muted card ──
  if (message.role === "tool") {
    return (
      <div className="flex justify-start">
        <div className="flex max-w-[80%] flex-col gap-1 rounded-[var(--radius-6)] border border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)] px-3 py-2">
          <p className="text-[11px] font-medium text-[var(--text-tertiary)]">
            {t.agentChat.toolResult}
          </p>
          <pre className="whitespace-pre-wrap text-xs text-[var(--text-secondary)]">
            {message.content}
          </pre>
        </div>
      </div>
    );
  }

  // ── Assistant message: white card, no avatar, with TRAE branding ──
  const showCursor = isStreaming;
  const hasReasoning = !!reasoning && reasoning.length > 0;
  const showContent = message.content.length > 0 || showCursor;

  return (
    <div className="flex justify-start">
      <div className="flex max-w-[85%] flex-col">
        {/* Main card */}
        <div className="rounded-[var(--radius-8)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-default)] px-4 py-3 shadow-sm">
          {/* Thinking process section */}
          <ThinkingProcess
            reasoning={reasoning}
            isStreaming={isStreaming ?? false}
          />

          {/* Main content */}
          {showContent ? (
            <>
              <MarkdownRenderer content={message.content} />
              {showCursor && (
                <span className="ml-0.5 inline-block h-3.5 w-[3px] animate-pulse bg-[var(--status-primary-default)] align-text-bottom" />
              )}
            </>
          ) : isStreaming && !hasReasoning ? (
            // No content, no reasoning yet — show thinking dots
            <span className="inline-flex items-center gap-1">
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--status-primary-default)]" />
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--status-primary-default)] [animation-delay:150ms]" />
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--status-primary-default)] [animation-delay:300ms]" />
            </span>
          ) : null}
        </div>

        {/* Action bar: only shown when not streaming */}
        {!isStreaming && message.content && (
          <div className="mt-1 flex items-center gap-1 px-1">
            <span className="mr-auto text-[10px] text-[var(--text-tertiary)]">
              {formatTime(message.created_at)}
            </span>
            <button
              type="button"
              onClick={handleCopy}
              className="flex size-6 items-center justify-center rounded-[var(--radius-4)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
              aria-label={t.agentChat.copyMessage}
            >
              {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
            </button>
            <button
              type="button"
              className="flex size-6 items-center justify-center rounded-[var(--radius-4)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
              aria-label="Thumbs up"
            >
              <ThumbsUp className="size-3" />
            </button>
            <button
              type="button"
              className="flex size-6 items-center justify-center rounded-[var(--radius-4)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
              aria-label="Thumbs down"
            >
              <ThumbsDown className="size-3" />
            </button>
            <button
              type="button"
              className="flex size-6 items-center justify-center rounded-[var(--radius-4)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
              aria-label={t.agentChat.regenerate}
            >
              <RotateCcw className="size-3" />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
