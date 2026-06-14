import { useCallback, useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAgentStore } from "@/stores/agent";
import * as agentService from "@/services/agent";
import type { Message, AgentEvent } from "@/types";

export function useAgent(sessionId: string | null) {
  const store = useAgentStore();
  const sessionIdRef = useRef(sessionId);
  sessionIdRef.current = sessionId;

  // Stable references to store actions
  const updateStreamingContent = useAgentStore((s) => s.updateStreamingContent);
  const clearStreamingContent = useAgentStore((s) => s.clearStreamingContent);
  const setStreaming = useAgentStore((s) => s.setStreaming);
  const setError = useAgentStore((s) => s.setError);
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const loadMessages = useAgentStore((s) => s.loadMessages);

  useEffect(() => {
    if (!sessionId) return;

    let cancelled = false;
    let unlistenFn: UnlistenFn | null = null;

    const setup = async () => {
      try {
        unlistenFn = await listen<AgentEvent>("agent-event", (event) => {
          if (cancelled) return;
          const payload = event.payload;

          console.log("[AgentEvent]", payload.type, "session:", payload.session_id, "current:", sessionIdRef.current);

          // Only process events for current session
          if (payload.session_id !== sessionIdRef.current) {
            console.log("[AgentEvent] Skipping - session mismatch");
            return;
          }

          switch (payload.type) {
            case "TurnStarted":
              console.log("[AgentEvent] TurnStarted - setting streaming=true");
              setStreaming(true);
              break;

            case "StreamDelta":
              if (payload.content) {
                updateStreamingContent(payload.content);
              }
              break;

            case "TurnCompleted":
              console.log("[AgentEvent] TurnCompleted");
              setStreaming(false);
              clearStreamingContent();
              // Reload messages from DB to get final state
              if (sessionIdRef.current) {
                loadMessages(sessionIdRef.current);
              }
              break;

            case "Error":
              console.log("[AgentEvent] Error:", payload.error);
              setStreaming(false);
              setError(payload.error || "Unknown error");
              break;

            case "ToolCallBegin":
              console.log("[ToolCall]", payload.tool, payload.args);
              break;

            case "ToolCallEnd":
              console.log("[ToolCallResult]", payload.tool_call_id, payload.output?.slice(0, 100));
              break;

            case "CompactionTriggered":
              console.log("[Compaction]", payload.session_id);
              break;
          }
        });
        console.log("[AgentEvent] Listener registered for session:", sessionId);
      } catch (err) {
        console.error("Failed to setup event listener:", err);
      }
    };

    setup();

    return () => {
      cancelled = true;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [sessionId, updateStreamingContent, clearStreamingContent, setStreaming, setError, loadMessages]);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!sessionId) {
        setError("请先创建或选择一个对话");
        return;
      }

      setError(null);
      clearStreamingContent();
      setStreaming(true);

      // Optimistically add user message
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        session_id: sessionId,
        role: "user",
        content,
        tool_calls: null,
        tool_results: null,
        token_count: null,
        created_at: new Date().toISOString(),
      };
      appendMessage(userMessage);

      try {
        await agentService.sendMessage({ sessionId, content });
      } catch (err) {
        setStreaming(false);
        setError(err instanceof Error ? err.message : "Failed to send message");
      }
    },
    [sessionId, appendMessage, clearStreamingContent, setStreaming, setError]
  );

  const approveTool = useCallback(
    async (toolCallId: string, approved: boolean) => {
      try {
        await agentService.approveTool(toolCallId, approved);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to approve tool");
      }
    },
    [setError]
  );

  const cancel = useCallback(() => {
    if (!sessionId) return;
    agentService.cancelAgent(sessionId);
    setStreaming(false);
  }, [sessionId, setStreaming]);

  return {
    messages: store.messages,
    streaming: store.streaming,
    streamingContent: store.streamingContent,
    error: store.error,
    loading: store.loading,
    sendMessage,
    approveTool,
    cancel,
  };
}
