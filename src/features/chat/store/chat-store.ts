import { create } from "zustand";
import { toast } from "sonner";
import type { Session, Message, PendingConfirmation } from "@/features/chat/types";
import * as sessionService from "@/features/session/services";

// 乐观更新辅助函数（P2 来自 AI Engineering 课程）
function generateTempId(): string {
  return `temp_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

/** 当前 turn 进行中的工具调用（流式展示用） */
export interface ActiveToolCall {
  id: string;
  name: string;
  status: "running" | "completed" | "error";
  startedAt: number;
}

interface AgentState {
  sessions: Session[];
  currentSessionId: string | null;
  messages: Message[];
  streaming: boolean;
  streamingContent: string;
  /** 当前 turn 流式累积的推理过程（reasoning_content / thinking_delta），与正文分离 */
  streamingReasoning: string;
  /** 当前 turn 进行中/已完成的工具调用列表（流式展示用，turn 结束后清空） */
  activeToolCalls: ActiveToolCall[];
  error: string | null;
  loading: boolean;
  /** SafetyGate 触发的待确认工具调用（null 表示无待处理） */
  pendingConfirmation: PendingConfirmation | null;
  submittingConfirmation: boolean;
  /** 用户主动清空会话标志（点击"新建任务"后为 true，阻止自动选会话） */
  pendingClear: boolean;
  loadSessions: (novelId?: string, workspaceId?: string) => Promise<void>;
  createSession: (novelId?: string, title?: string, workspaceId?: string) => Promise<Session>;
  switchSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadMessages: (sessionId: string) => Promise<void>;
  /** 清空当前会话引用（进入空白对话页，不创建新会话） */
  clearCurrentSession: () => void;
  appendMessage: (message: Message) => void;
  replaceMessages: (messages: Message[]) => void;
  updateStreamingContent: (delta: string) => void;
  clearStreamingContent: () => void;
  updateStreamingReasoning: (delta: string) => void;
  clearStreamingReasoning: () => void;
  /** 新增一个工具调用（toolCallStart 时调用） */
  addToolCallStart: (call: { id: string; name: string }) => void;
  /** 标记某个工具调用完成（toolCallEnd 时调用） */
  updateToolCallEnd: (id: string, status?: "completed" | "error") => void;
  /** 清空所有进行中的工具调用（turn 结束时） */
  clearActiveToolCalls: () => void;
  setStreaming: (streaming: boolean) => void;
  setError: (error: string | null) => void;
  setPendingConfirmation: (pending: PendingConfirmation | null) => void;
  setSubmittingConfirmation: (submitting: boolean) => void;
  /** 用户确认/取消后清理待确认状态 */
  respondConfirmation: () => void;
  reset: () => void;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  sessions: [],
  currentSessionId: null,
  messages: [],
  streaming: false,
  streamingContent: "",
  streamingReasoning: "",
  activeToolCalls: [],
  error: null,
  loading: false,
  pendingConfirmation: null,
  submittingConfirmation: false,
  pendingClear: false,

  loadSessions: async (novelId?: string, workspaceId?: string) => {
    set({ loading: true, error: null });
    try {
      const sessions = await sessionService.listSessions(novelId, workspaceId);
      set({ sessions, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load sessions";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  // 乐观创建 session（P2 - 立即更新 UI）
  createSession: async (novelId?: string, title?: string, workspaceId?: string) => {
    const tempId = generateTempId();
    const optimisticSession: Session = {
      id: tempId,
      title: title || "New Session",
      novel_id: novelId || null,
      workspace_id: workspaceId || null,
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
      activeToolCalls: [],
      streaming: false,
      error: null,
      pendingClear: false,
    }));

    try {
      const session = await sessionService.createSession(novelId, title, workspaceId);
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
    set({ currentSessionId: sessionId, loading: true, error: null, streaming: false, streamingContent: "", streamingReasoning: "", activeToolCalls: [], pendingConfirmation: null, submittingConfirmation: false, pendingClear: false });
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

  clearCurrentSession: () => {
    set({
      currentSessionId: null,
      messages: [],
      streaming: false,
      streamingContent: "",
      streamingReasoning: "",
      activeToolCalls: [],
      error: null,
      pendingConfirmation: null,
      submittingConfirmation: false,
      pendingClear: true,
    });
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

  addToolCallStart: (call) => {
    set((state) => ({
      activeToolCalls: [
        ...state.activeToolCalls,
        { id: call.id, name: call.name, status: "running", startedAt: Date.now() },
      ],
    }));
  },

  updateToolCallEnd: (id, status = "completed") => {
    set((state) => ({
      activeToolCalls: state.activeToolCalls.map((c) =>
        c.id === id ? { ...c, status } : c,
      ),
    }));
  },

  clearActiveToolCalls: () => {
    set({ activeToolCalls: [] });
  },

  setStreaming: (streaming: boolean) => {
    set({ streaming });
  },

  setError: (error: string | null) => {
    set({ error });
  },

  setPendingConfirmation: (pending) => {
    set({ pendingConfirmation: pending, submittingConfirmation: false });
  },

  setSubmittingConfirmation: (submitting) => {
    set({ submittingConfirmation: submitting });
  },

  respondConfirmation: () => {
    set({ pendingConfirmation: null, submittingConfirmation: false });
  },

  reset: () => {
    set({
      messages: [],
      streaming: false,
      streamingContent: "",
      streamingReasoning: "",
      activeToolCalls: [],
      error: null,
      pendingConfirmation: null,
      submittingConfirmation: false,
    });
  },
}));
