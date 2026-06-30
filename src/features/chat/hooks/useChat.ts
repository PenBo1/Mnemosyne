import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useAgentSession } from "@/features/chat/hooks";
import { useAgentStore } from "@/stores/agent";
import * as agentService from "@/features/chat/services";
import type { Message, Session } from "@/shared/types";

/**
 * 页面级 Chat hook：组合会话列表 + 当前会话消息 + 流式事件订阅 + 发消息流程。
 *
 * - 会话列表 / 当前 sessionId 通过 store selector（自动加载）
 * - 消息 / 流式状态 / 事件订阅通过 useAgent(currentSessionId)
 * - sendMessage 自行实现，处理"无 session 时先创建"的流程
 *   （useAgent.sendMessage 在 sessionId=null 时会报错，故不复用）
 */
export function useChat() {
  const sessions = useAgentStore((s) => s.sessions);
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const loading = useAgentStore((s) => s.loading);
  const loadSessions = useAgentStore((s) => s.loadSessions);
  const switchSession = useAgentStore((s) => s.switchSession);
  const createSession = useAgentStore((s) => s.createSession);
  const deleteSession = useAgentStore((s) => s.deleteSession);

  // 订阅 agent-event + 读 messages/streaming/error；cancel 复用 useAgentSession
  const { messages, streaming, streamingContent, streamingReasoning, error, cancel } =
    useAgentSession(currentSessionId);

  // 写操作 action（通过 selector，避免整树订阅）
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const setStreaming = useAgentStore((s) => s.setStreaming);
  const clearStreamingContent = useAgentStore((s) => s.clearStreamingContent);
  const setError = useAgentStore((s) => s.setError);

  // 跟踪 latest currentSessionId，避免 sendMessage 闭包陈旧
  const currentSessionIdRef = useRef(currentSessionId);
  currentSessionIdRef.current = currentSessionId;

  // 首次挂载加载会话列表
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // 进入页面时若已有会话但未选中，自动选第一个
  useEffect(() => {
    if (sessions.length > 0 && !currentSessionId) {
      switchSession(sessions[0].id);
    }
  }, [sessions, currentSessionId, switchSession]);

  const sendMessage = useCallback(
    async (content: string) => {
      const trimmed = content.trim();
      if (!trimmed) return;

      // 无 session 时先创建（拿到真实 id 后再发）
      let sid = currentSessionIdRef.current;
      if (!sid) {
        try {
          const session = await createSession();
          sid = session.id;
        } catch {
          return; // createSession 已通过 toast 报错
        }
      }

      setError(null);
      clearStreamingContent();
      setStreaming(true);

      // 乐观追加 user 消息
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        session_id: sid,
        role: "user",
        content: trimmed,
        tool_calls: null,
        tool_results: null,
        token_count: null,
        created_at: new Date().toISOString(),
      };
      appendMessage(userMessage);

      try {
        await agentService.sendMessage({ sessionId: sid, content: trimmed });
      } catch (err) {
        setStreaming(false);
        const msg = err instanceof Error ? err.message : "Failed to send message";
        setError(msg);
        toast.error(msg);
      }
    },
    [createSession, appendMessage, clearStreamingContent, setStreaming, setError]
  );

  const handleNewSession = useCallback(async () => {
    // 当前已是空会话则不重复创建
    if (currentSessionId && messages.length === 0) return;
    try {
      await createSession();
    } catch {
      // 已 toast
    }
  }, [createSession, currentSessionId, messages.length]);

  const handleDeleteSession = useCallback(async () => {
    if (!currentSessionId) return;
    await deleteSession(currentSessionId);
  }, [currentSessionId, deleteSession]);

  const activeSession: Session | null =
    sessions.find((s) => s.id === currentSessionId) ?? null;

  return {
    sessions,
    currentSessionId,
    activeSession,
    messages,
    streaming,
    streamingContent,
    streamingReasoning,
    error,
    loading,
    sendMessage,
    cancel,
    switchSession,
    handleNewSession,
    handleDeleteSession,
  };
}
