import { useCallback, useEffect, useRef, useState } from "react";

const DEFAULT_RESET_DELAY_MS = 2000;

/**
 * 复制到剪贴板 + "已复制"反馈状态。
 *
 * 统一替代 CodeBlock / MessageBubble 等组件中重复的
 * `navigator.clipboard.writeText + setTimeout(setCopied(false))` 样板。
 *
 * 自动在卸载时清理定时器，避免 "setState on unmounted component" 警告。
 */
export function useCopyFeedback(resetDelayMs = DEFAULT_RESET_DELAY_MS) {
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const copy = useCallback(
    (text: string) => {
      navigator.clipboard.writeText(text).then(() => {
        setCopied(true);
        if (timerRef.current) clearTimeout(timerRef.current);
        timerRef.current = setTimeout(() => setCopied(false), resetDelayMs);
      });
    },
    [resetDelayMs],
  );

  return { copied, copy } as const;
}
