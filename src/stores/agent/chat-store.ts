import { create } from "zustand";
import { toast } from "sonner";
import type { Session, Message } from "@/shared/types";
import * as sessionService from "@/features/session/services";

// 乐观更新辅助函数（P2 来自 AI Engineering 课程）
function generateTempId(): string {
  return `temp_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

interface AgentState {
  sessions: Session[];
  currentSessionId: string | null;
  messages: Message[];
  streaming: boolean;
  streamingContent: string;
  /** 当前 turn 流式累积的推理过程（reasoning_content / thinking_delta），与正文分离 */
  streamingReasoning: string;
  error: string | null;
  loading: boolean;
  loadSessions: (novelId?: string) => Promise<void>;
  createSession: (novelId?: string, title?: string) => Promise<Session>;
  switchSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadMessages: (sessionId: string) => Promise<void>;
  appendMessage: (message: Message) => void;
  replaceMessages: (messages: Message[]) => void;
  updateStreamingContent: (delta: string) => void;
  clearStreamingContent: () => void;
  updateStreamingReasoning: (delta: string) => void;
  clearStreamingReasoning: () => void;
  setStreaming: (streaming: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  sessions: [],
  currentSessionId: null,
  messages: [],
  streaming: false,
  streamingContent: "",
  streamingReasoning: "",
  error: null,
  loading: false,

  loadSessions: async (novelId?: string) => {
    set({ loading: true, error: null });
    try {
      const sessions = await sessionService.listSessions(novelId);
      set({ sessions, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load sessions";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  // 乐观创建 session（P2 - 立即更新 UI）
  createSession: async (novelId?: string, title?: string) => {
    const tempId = generateTempId();
    const optimisticSession: Session = {
      id: tempId,
      title: title || "New Session",
      novel_id: novelId || null,
      session_type: "chat",
      summary: null,
      message_count: 0,
      input_tokens: 0,
      output_tokens: 0,
      cost: 0,
      status: "active",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    // 乐观更新：立即在 UI 中显示
    set((state) => ({
      sessions: [optimisticSession, ...state.sessions],
      currentSessionId: tempId,
      messages: [],
      streamingContent: "",
      streamingReasoning: "",
      streaming: false,
      error: null,
    }));

    try {
      const session = await sessionService.createSession(novelId, title);
      // 用真实数据替换乐观数据
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === tempId ? session : s
        ),
        currentSessionId: session.id,
      }));
      return session;
    } catch (err) {
      // 回滚乐观更新
      set((state) => ({
        sessions: state.sessions.filter((s) => s.id !== tempId),
        currentSessionId: null,
        error: err instanceof Error ? err.message : "Failed to create session",
      }));
      toast.error("Failed to create session");
      throw err;
    }
  },

  switchSession: async (sessionId: string) => {
    set({ currentSessionId: sessionId, loading: true, error: null, streaming: false, streamingContent: "", streamingReasoning: "" });
    try {
      const messages = await sessionService.listMessages(sessionId);
      set({ messages, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load messages";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  // 乐观删除 session（P2 - 立即更新 UI）
  deleteSession: async (sessionId: string) => {
    // 乐观更新：立即从 UI 中移除
    const previousSessions = get().sessions;
    const previousCurrentId = get().currentSessionId;

    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== sessionId),
      currentSessionId:
        state.currentSessionId === sessionId ? null : state.currentSessionId,
      messages: state.currentSessionId === sessionId ? [] : state.messages,
    }));

    try {
      await sessionService.deleteSession(sessionId);
    } catch (err) {
      // 失败时回滚
      set({
        sessions: previousSessions,
        currentSessionId: previousCurrentId,
        error: err instanceof Error ? err.message : "Failed to delete session",
      });
      toast.error("Failed to delete session");
    }
  },

  loadMessages: async (sessionId: string) => {
    try {
      const messages = await sessionService.listMessages(sessionId);
      set({ messages });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load messages";
      toast.error(message);
    }
  },

  appendMessage: (message: Message) => {
    set((state) => ({
      messages: [...state.messages, message],
    }));
  },

  replaceMessages: (messages: Message[]) => {
    set({ messages });
  },

  updateStreamingContent: (delta: string) => {
    set((state) => ({
      streamingContent: state.streamingContent + delta,
    }));
  },

  clearStreamingContent: () => {
    set({ streamingContent: "" });
  },

  updateStreamingReasoning: (delta: string) => {
    set((state) => ({
      streamingReasoning: state.streamingReasoning + delta,
    }));
  },

  clearStreamingReasoning: () => {
    set({ streamingReasoning: "" });
  },

  setStreaming: (streaming: boolean) => {
    set({ streaming });
  },

  setError: (error: string | null) => {
    set({ error });
  },

  reset: () => {
    set({
      messages: [],
      streaming: false,
      streamingContent: "",
      streamingReasoning: "",
      error: null,
    });
  },
}));
