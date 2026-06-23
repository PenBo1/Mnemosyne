import { useCallback, useRef, useState, useEffect } from "react";
import { AutoResizeTextarea } from "./AutoResizeTextarea";
import { ComposerControls } from "./ComposerControls";
import { useInputHistory } from "./hooks/useInputHistory";
import { useI18n } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import type { ChatComposerProps } from "@/types/chat";

export function ChatComposer({
  value,
  onChange,
  onSubmit,
  onCancel,
  disabled,
  streaming,
  placeholder,
  maxRows = 5,
  minRows = 1,
  autoFocus,
  className,
}: ChatComposerProps) {
  const { t } = useI18n();
  const { handleHistoryUp, handleHistoryDown, handleHistorySubmit, handleHistoryReset } = useInputHistory();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [composing, setComposing] = useState(false);

  // Track IME composition state
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    const handleCompositionStart = () => setComposing(true);
    const handleCompositionEnd = () => setComposing(false);

    textarea.addEventListener("compositionstart", handleCompositionStart);
    textarea.addEventListener("compositionend", handleCompositionEnd);

    return () => {
      textarea.removeEventListener("compositionstart", handleCompositionStart);
      textarea.removeEventListener("compositionend", handleCompositionEnd);
    };
  }, []);

  const handleSubmit = useCallback(() => {
    if (!value.trim() || disabled || streaming) return;
    handleHistorySubmit(value.trim());
    onSubmit();
  }, [value, disabled, streaming, handleHistorySubmit, onSubmit]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Don't handle during IME composition
      if (composing) return;

      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }

      // History navigation (Up/Down when at start/end of input)
      if (e.key === "ArrowUp") {
        const textarea = e.currentTarget;
        const atStart = textarea.selectionStart === 0 && textarea.selectionEnd === 0;
        if (atStart) {
          e.preventDefault();
          const previous = handleHistoryUp(value);
          if (previous !== null) {
            onChange(previous);
          }
        }
      }

      if (e.key === "ArrowDown") {
        const textarea = e.currentTarget;
        const atEnd = textarea.selectionStart === textarea.value.length && textarea.selectionEnd === textarea.value.length;
        if (atEnd) {
          e.preventDefault();
          const next = handleHistoryDown();
          onChange(next ?? "");
        }
      }

      // Escape to reset history navigation
      if (e.key === "Escape") {
        handleHistoryReset();
      }
    },
    [composing, handleSubmit, handleHistoryUp, handleHistoryDown, handleHistoryReset, value, onChange]
  );

  const canSubmit = value.trim().length > 0 && !disabled && !streaming;
  const placeholderText = placeholder ?? t.agentChat.placeholder;

  return (
    <div className={cn("relative", className)}>
      <div className="flex gap-2 items-end">
        <div className="flex-1 min-w-0">
          <AutoResizeTextarea
            ref={textareaRef}
            value={value}
            onChange={onChange}
            onKeyDown={handleKeyDown}
            placeholder={placeholderText}
            disabled={disabled || streaming}
            minRows={minRows}
            maxRows={maxRows}
            autoFocus={autoFocus}
            aria-label={placeholderText}
          />
        </div>
        <ComposerControls
          streaming={streaming}
          disabled={disabled}
          canSubmit={canSubmit}
          onSubmit={handleSubmit}
          onCancel={onCancel}
        />
      </div>
    </div>
  );
}