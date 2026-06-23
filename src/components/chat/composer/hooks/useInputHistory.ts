import { useCallback, useRef, useState } from "react";
import { useAgentStore } from "@/stores/agent";

export function useInputHistory() {
  const inputHistory = useAgentStore((s) => s.inputHistory);
  const addToHistory = useAgentStore((s) => s.addToHistory);
  const browseHistoryBack = useAgentStore((s) => s.browseHistoryBack);
  const browseHistoryForward = useAgentStore((s) => s.browseHistoryForward);
  const resetHistoryNavigation = useAgentStore((s) => s.resetHistoryNavigation);

  // Track the draft before history navigation started
  const draftRef = useRef<string>("");
  const [isNavigating, setIsNavigating] = useState(false);

  const handleHistoryUp = useCallback(
    (currentInput: string) => {
      if (!isNavigating) {
        // Save current draft before navigating
        draftRef.current = currentInput;
        setIsNavigating(true);
      }

      const previous = browseHistoryBack();
      return previous;
    },
    [browseHistoryBack, isNavigating]
  );

  const handleHistoryDown = useCallback(() => {
    const next = browseHistoryForward();
    if (next === null && isNavigating) {
      // Reached the end, restore draft
      setIsNavigating(false);
      return draftRef.current;
    }
    return next;
  }, [browseHistoryForward, isNavigating]);

  const handleHistoryReset = useCallback(() => {
    setIsNavigating(false);
    resetHistoryNavigation();
  }, [resetHistoryNavigation]);

  const handleHistorySubmit = useCallback(
    (content: string) => {
      addToHistory(content);
      handleHistoryReset();
    },
    [addToHistory, handleHistoryReset]
  );

  return {
    inputHistory,
    handleHistoryUp,
    handleHistoryDown,
    handleHistoryReset,
    handleHistorySubmit,
    isNavigating,
  };
}