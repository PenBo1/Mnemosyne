import { useEffect, useRef } from "react";
import { ArrowUp, StopCircle } from "lucide-react";
import { useI18n } from "@/shared/i18n";

export function ChatInput({
  value,
  onChange,
  onSubmit,
  onCancel,
  streaming,
}: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  streaming: boolean;
}) {
  const { t } = useI18n();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // 自适应高度
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(Math.max(el.scrollHeight, 56), 200)}px`;
  }, [value]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onSubmit();
    }
  };

  const canSend = value.trim().length > 0 && !streaming;

  return (
    <div className="border-t border-border bg-background px-4 pb-4 pt-2">
      <div className="mx-auto max-w-3xl">
        <div
          className={`flex flex-col rounded-2xl border bg-card shadow-sm transition-[border-color,box-shadow] ${
            value.trim()
              ? "border-border focus-within:border-primary/40 focus-within:shadow-md"
              : "border-border"
          }`}
        >
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.agentChat.placeholder}
            rows={1}
            className="w-full resize-none bg-transparent px-4 pt-3.5 pb-2 text-sm leading-[1.6] text-foreground outline-none placeholder:text-muted-foreground/50"
            style={{ minHeight: "56px", maxHeight: "200px" }}
          />
          <div className="flex items-center justify-end border-t border-border/50 px-3 py-2">
            {streaming ? (
              <button
                type="button"
                onClick={onCancel}
                className="flex size-8 items-center justify-center rounded-xl bg-destructive/10 text-destructive transition-colors hover:bg-destructive/20"
                aria-label={t.agentChat.stop}
              >
                <StopCircle className="size-4" />
              </button>
            ) : (
              <button
                type="button"
                onClick={onSubmit}
                disabled={!canSend}
                className={`flex size-8 items-center justify-center rounded-xl transition-all ${
                  canSend
                    ? "bg-primary text-primary-foreground shadow-sm hover:opacity-90 active:scale-95"
                    : "cursor-not-allowed bg-muted text-muted-foreground/25"
                }`}
                aria-label={t.agentChat.send}
              >
                <ArrowUp className="size-4" />
              </button>
            )}
          </div>
        </div>
        <div className="mt-1.5 flex items-center justify-center gap-1 text-[10px] text-muted-foreground/40">
          <kbd className="rounded border border-border/30 bg-card px-1 py-px font-mono">
            Enter
          </kbd>
          <span>{t.common.send}</span>
          <span>·</span>
          <kbd className="rounded border border-border/30 bg-card px-1 py-px font-mono">
            Shift+Enter
          </kbd>
          <span>{t.agentChat.newline}</span>
        </div>
      </div>
    </div>
  );
}
