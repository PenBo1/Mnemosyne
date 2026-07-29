import { create } from "zustand";
import { toast } from "sonner";
import type { Session, Message, PendingConfirmation } from "@/types/session";
import {
  createSession as sessionCreate,
  listSessions as sessionList,
  listMessages as sessionListMessages,
  deleteSession as sessionDelete,
} from "@/features/session/services";
import { getTranslations } from "@/locales/i18n-store";
import type { PipelineState } from "@/features/agent/components/PipelineProgress";

// 乐观更新辅助函数（P2 来自 AI Engineering 课程）
function generateTempId(): string {
  return `temp_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

// messages 数组软上限：超过阈值时压缩，避免前端缓存无限增长。
// 保留首条 system 消息（系统上下文）+ 最近 MESSAGES_KEEP_RECENT 条对话。
const MESSAGES_SOFT_LIMIT = 500;
const MESSAGES_KEEP_RECENT = 50;

/** 超过软上限时压缩：保留首条 system 消息 + 最近 N 条。未超阈值原样返回。 */
function compactMessagesIfNeeded(messages: Message[]): Message[] {
  if (messages.length <= MESSAGES_SOFT_LIMIT) return messages;
  const recent = messages.slice(-MESSAGES_KEEP_RECENT);
  const first = messages[0];
  // messages.length > 500 > 50，首条必不在 recent 内；仅当为 system 时保留
  if (first && first.role === "system") {
    return [first, ...recent];
  }
  return recent;
}

/** 当前 turn 进行中的工具调用（流式展示用） */
export interface ActiveToolCall {
  id: string;
  name: string;
  status: "running" | "completed" | "error";
  /** 工具调用开始时在文本 buffer 中的位置（用于按顺序渲染） */
  startPosition: number;
  endPosition?: number;
  /** 工具调用参数（流式累积，toolCallDelta 事件追加） */
  args?: string;
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
  /** Pipeline 进度状态（用于进度可视化） */
  pipelineState: PipelineState | null;
  loadSessions: (novelId?: string, workspaceId?: string) => Promise<void>;
  createSession: (novelId?: string, title?: string, workspaceId?: string) => Promise<Session>;
  switchSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadMessages: (sessionId: string) => Promise<void>;
  /** 清空当前会话引用（进入空白对话页，不创建新会话） */
  clearCurrentSession: () => void;
  /** 检查是否有正在运行的任务（streaming 或 pending confirmation） */
  isAgentRunning: () => boolean;
  /** 强制停止当前运行的任务 */
  forceStop: () => void;
  appendMessage: (message: Message) => void;
  replaceMessages: (messages: Message[]) => void;
  updateStreamingContent: (delta: string) => void;
  clearStreamingContent: () => void;
  updateStreamingReasoning: (delta: string) => void;
  clearStreamingReasoning: () => void;
  /** 新增一个工具调用（toolCallStart 时调用） */
  addToolCallStart: (call: { id: string; name: string; startPosition: number }) => void;
  /** 标记某个工具调用完成（toolCallEnd 时调用） */
  updateToolCallEnd: (id: string, status?: "completed" | "error") => void;
  /** 追加工具调用参数（toolCallDelta 时调用，累积 argsDelta） */
  appendToolCallArgs: (id: string, argsDelta: string) => void;
  /** 清空所有进行中的工具调用（turn 结束时） */
  clearActiveToolCalls: () => void;
  setStreaming: (streaming: boolean) => void;
  setError: (error: string | null) => void;
  setPendingConfirmation: (pending: PendingConfirmation | null) => void;
  setSubmittingConfirmation: (submitting: boolean) => void;
  /** 用户确认/取消后清理待确认状态 */
  respondConfirmation: () => void;
  /** 设置 Pipeline 进度状态 */
  setPipelineState: (state: PipelineState | null) => void;
  /** 清空 Pipeline 进度状态 */
  clearPipelineState: () => void;
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
  pipelineState: null,

  loadSessions: async (novelId?: string, workspaceId?: string) => {
    set({ loading: true, error: null });
    try {
      const sessions = await sessionList(novelId, workspaceId);
      set({ sessions, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : getTranslations().chat.errors.failedToLoadSessions;
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
      const session = await sessionCreate(novelId, title, workspaceId);
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
        error: err instanceof Error ? err.message : getTranslations().chat.errors.failedToCreateSession,
      }));
      toast.error(getTranslations().chat.errors.failedToCreateSession);
      throw err;
    }
  },

  switchSession: async (sessionId: string) => {
    // 如果 agent 正在运行，先停止
    const state = get();
    if (state.streaming || state.pendingConfirmation) {
      // 标记停止，让流式处理逻辑感知
      set({ streaming: false });
    }
    set({ currentSessionId: sessionId, loading: true, error: null, streaming: false, streamingContent: "", streamingReasoning: "", activeToolCalls: [], pendingConfirmation: null, submittingConfirmation: false, pendingClear: false });
    try {
      const messages = await sessionListMessages(sessionId);
      set({ messages, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : getTranslations().chat.errors.failedToLoadMessages;
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
      await sessionDelete(sessionId);
    } catch (err) {
      // 失败时回滚
      set({
        sessions: previousSessions,
        currentSessionId: previousCurrentId,
        error: err instanceof Error ? err.message : getTranslations().chat.errors.failedToDeleteSession,
      });
      toast.error(getTranslations().chat.errors.failedToDeleteSession);
    }
  },

  loadMessages: async (sessionId: string) => {
    try {
      const messages = await sessionListMessages(sessionId);
      set({ messages });
    } catch (err) {
      const message = err instanceof Error ? err.message : getTranslations().chat.errors.failedToLoadMessages;
      toast.error(message);
    }
  },

  clearCurrentSession: () => {
    // 如果 agent 正在运行，先停止
    const state = get();
    if (state.streaming || state.pendingConfirmation) {
      set({ streaming: false });
    }
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

  isAgentRunning: () => {
    const state = get();
    return state.streaming || state.pendingConfirmation !== null;
  },

  forceStop: () => {
    set({
      streaming: false,
      streamingContent: "",
      streamingReasoning: "",
      activeToolCalls: [],
      pendingConfirmation: null,
    });
  },

  appendMessage: (message: Message) => {
    set((state) => {
      const next = [...state.messages, message];
      return { messages: compactMessagesIfNeeded(next) };
    });
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
        { id: call.id, name: call.name, status: "running", startPosition: call.startPosition, startedAt: Date.now() },
      ],
    }));
  },

  updateToolCallEnd: (id, status = "completed") => {
    const currentContentLength = get().streamingContent.length;
    set((state) => ({
      activeToolCalls: state.activeToolCalls.map((c) =>
        c.id === id ? { ...c, status, endPosition: currentContentLength } : c,
      ),
    }));
  },

  appendToolCallArgs: (id, argsDelta) => {
    set((state) => ({
      activeToolCalls: state.activeToolCalls.map((c) =>
        c.id === id ? { ...c, args: (c.args ?? "") + argsDelta } : c,
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

  setPipelineState: (state) => {
    set({ pipelineState: state });
  },

  clearPipelineState: () => {
    set({ pipelineState: null });
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
      pipelineState: null,
    });
  },
}));
