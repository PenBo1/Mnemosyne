import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useAgentStore } from "@/stores/agent";
import * as agentService from "@/features/chat/services";
import { onAgentEvent } from "@/features/chat/services";

export function useAgentSession(sessionId: string | null) {
  const sessionIdRef = useRef(sessionId);
  sessionIdRef.current = sessionId;

  const messages = useAgentStore((s) => s.messages);
  const streaming = useAgentStore((s) => s.streaming);
  const streamingContent = useAgentStore((s) => s.streamingContent);
  const streamingReasoning = useAgentStore((s) => s.streamingReasoning);
  const error = useAgentStore((s) => s.error);
  const loading = useAgentStore((s) => s.loading);
  const pendingConfirmation = useAgentStore((s) => s.pendingConfirmation);
  const submittingConfirmation = useAgentStore((s) => s.submittingConfirmation);
  const updateStreamingContent = useAgentStore((s) => s.updateStreamingContent);
  const clearStreamingContent = useAgentStore((s) => s.clearStreamingContent);
  const updateStreamingReasoning = useAgentStore((s) => s.updateStreamingReasoning);
  const clearStreamingReasoning = useAgentStore((s) => s.clearStreamingReasoning);
  const setStreaming = useAgentStore((s) => s.setStreaming);
  const setError = useAgentStore((s) => s.setError);
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const loadMessages = useAgentStore((s) => s.loadMessages);
  const setPendingConfirmation = useAgentStore((s) => s.setPendingConfirmation);
  const setSubmittingConfirmation = useAgentStore((s) => s.setSubmittingConfirmation);

  useEffect(() => {
    if (!sessionId) return;

    let cancelled = false;
    let unlistenFn: (() => void) | null = null;

    const setup = async () => {
      try {
        unlistenFn = await onAgentEvent(sessionId, (payload) => {
          if (cancelled) return;

          switch (payload.type) {
            case "TurnStarted":
              setStreaming(true);
              clearStreamingReasoning();
              // 新 turn 开始时清除上一轮可能残留的确认请求
              setPendingConfirmation(null);
              break;

            case "StreamDelta":
              if (payload.content) {
                updateStreamingContent(payload.content);
              }
              break;

            case "ReasoningDelta":
              if (payload.content) {
                updateStreamingReasoning(payload.content);
              }
              break;

            case "TurnCompleted":
              setStreaming(false);
              clearStreamingContent();
              clearStreamingReasoning();
              setPendingConfirmation(null);
              if (sessionIdRef.current) {
                loadMessages(sessionIdRef.current);
              }
              break;

            case "Error":
              setStreaming(false);
              setError(payload.error || "Unknown error");
              setPendingConfirmation(null);
              toast.error(payload.error || "Unknown error");
              break;

            case "ConfirmationRequired": {
              // SafetyGate 请求用户确认：写入 store 触发对话框
              if (!payload.tool_call_id || !payload.tool || !payload.risk_level) {
                break;
              }
              setPendingConfirmation({
                toolCallId: payload.tool_call_id,
                tool: payload.tool,
                description: payload.description ?? "",
                details: payload.details ?? "",
                riskLevel: payload.risk_level,
              });
              break;
            }

            case "ToolCallBegin":
            case "ToolCallEnd":
            case "CompactionTriggered":
              break;
          }
        });
      } catch {
        setError("Failed to setup event listener");
        toast.error("Failed to setup event listener");
      }
    };

    setup();

    return () => {
      cancelled = true;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [sessionId, updateStreamingContent, clearStreamingContent, updateStreamingReasoning, clearStreamingReasoning, setStreaming, setError, loadMessages, setPendingConfirmation]);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!sessionId) {
        setError("Please create or select a session first");
        toast.error("Please create or select a session first");
        return;
      }

      setError(null);
      clearStreamingContent();
      setStreaming(true);

      const userMessage = {
        id: `temp-${Date.now()}`,
        session_id: sessionId,
        role: "user" as const,
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
        toast.error(err instanceof Error ? err.message : "Failed to send message");
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
        toast.error(err instanceof Error ? err.message : "Failed to approve tool");
      }
    },
    [setError]
  );

  /**
   * 响应 SafetyGate 确认请求。提交后立即清空 pendingConfirmation（后端会继续推进 turn）。
   */
  const respondConfirmation = useCallback(
    async (params: {
      approved: boolean;
      autoApproveSimilar: boolean;
      modifiedArgs?: string;
    }) => {
      const sid = sessionIdRef.current;
      if (!sid || !pendingConfirmation) return;
      setSubmittingConfirmation(true);
      try {
        await agentService.respondConfirmation({
          sessionId: sid,
          toolCallId: pendingConfirmation.toolCallId,
          approved: params.approved,
          autoApproveSimilar: params.autoApproveSimilar,
          modifiedArgs: params.modifiedArgs,
        });
        setPendingConfirmation(null);
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to respond";
        setError(msg);
        toast.error(msg);
      } finally {
        setSubmittingConfirmation(false);
      }
    },
    [pendingConfirmation, setError, setPendingConfirmation, setSubmittingConfirmation]
  );

  const cancel = useCallback(() => {
    if (!sessionId) return;
    agentService.cancelAgent(sessionId);
    setStreaming(false);
    setPendingConfirmation(null);
  }, [sessionId, setStreaming, setPendingConfirmation]);

  return {
    messages,
    streaming,
    streamingContent,
    streamingReasoning,
    error,
    loading,
    pendingConfirmation,
    submittingConfirmation,
    sendMessage,
    approveTool,
    respondConfirmation,
    cancel,
  };
}
