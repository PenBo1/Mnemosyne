import { useRef, useEffect, useCallback } from "react";

export interface ScrollRef {
  scrollToBottom: () => void;
  checkIsAtBottom: () => boolean;
}

export function useScroll() {
  const scrollRef = useRef<ScrollRef | null>(null);
  const rafRef = useRef<number | null>(null);

  const scrollToBottom = useCallback((force = false) => {
    const container = scrollRef.current;
    if (!container) return;
    if (!force && !container.checkIsAtBottom()) return;
    container.scrollToBottom();
  }, []);

  const scrollToBottomRAF = useCallback(() => {
    if (rafRef.current !== null) return;
    rafRef.current = requestAnimationFrame(() => {
      rafRef.current = null;
      scrollToBottom(false);
    });
  }, [scrollToBottom]);

  useEffect(() => {
    scrollToBottom(true);
  }, []);

  useEffect(() => {
    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, []);

  return {
    scrollRef,
    scrollToBottom,
    scrollToBottomRAF,
  };
}