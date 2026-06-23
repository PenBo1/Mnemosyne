import { useRef, useEffect, useCallback, forwardRef } from "react";
import { cn } from "@/lib/utils";

interface AutoResizeTextareaProps {
  value: string;
  onChange: (value: string) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  disabled?: boolean;
  minRows?: number;
  maxRows?: number;
  autoFocus?: boolean;
  className?: string;
  id?: string;
  name?: string;
  "aria-label"?: string;
}

const LINE_HEIGHT = 22;
const PADDING_Y = 12;

export const AutoResizeTextarea = forwardRef<HTMLTextAreaElement, AutoResizeTextareaProps>(
  function AutoResizeTextarea(
    { value, onChange, onKeyDown, placeholder, disabled, minRows = 1, maxRows = 5, autoFocus, className, id, name, "aria-label": ariaLabel },
    ref
  ) {
    const textareaRef = useRef<HTMLTextAreaElement | null>(null);
    const minHeight = minRows * LINE_HEIGHT + PADDING_Y;
    const maxHeight = maxRows * LINE_HEIGHT + PADDING_Y;

    const adjustHeight = useCallback(() => {
      const textarea = textareaRef.current;
      if (!textarea) return;

      // Reset height to get accurate scrollHeight
      textarea.style.height = `${minHeight}px`;
      const scrollHeight = textarea.scrollHeight;

      // Set height within bounds
      const newHeight = Math.min(Math.max(scrollHeight, minHeight), maxHeight);
      textarea.style.height = `${newHeight}px`;

      // Enable scroll when exceeding max
      textarea.style.overflowY = scrollHeight > maxHeight ? "auto" : "hidden";
    }, [minHeight, maxHeight]);

    useEffect(() => {
      adjustHeight();
    }, [value, adjustHeight]);

    useEffect(() => {
      if (autoFocus && textareaRef.current) {
        textareaRef.current.focus();
      }
    }, [autoFocus]);

    // Combine refs
    const combinedRef = useCallback((el: HTMLTextAreaElement | null) => {
      textareaRef.current = el;
      if (typeof ref === "function") {
        ref(el);
      } else if (ref) {
        ref.current = el;
      }
    }, [ref]);

    return (
      <textarea
        ref={combinedRef}
        id={id}
        name={name}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        autoFocus={autoFocus}
        aria-label={ariaLabel}
        className={cn(
          "w-full resize-none overflow-y-hidden rounded-lg border border-input bg-background px-3 py-2 text-sm outline-none transition-colors",
          "placeholder:text-muted-foreground",
          "focus:border-ring focus:ring-2 focus:ring-ring/30",
          "disabled:cursor-not-allowed disabled:opacity-50",
          className
        )}
        style={{
          minHeight: `${minHeight}px`,
          maxHeight: `${maxHeight}px`,
          lineHeight: `${LINE_HEIGHT}px`,
        }}
      />
    );
  }
);