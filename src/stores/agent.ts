import { create } from "zustand";
import { toast } from "sonner";
import type { Session, Message } from "@/types";
import * as sessionService from "@/services/session";

// Optimistic update helper (P2 from AI Engineering curriculum)
function generateTempId(): string {
  return `temp_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

interface AgentState {
  sessions: Session[];
  currentSessionId: string | null;
  messages: Message[];
  streaming: boolean;
  streamingContent: string;
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
  setStreaming: (streaming: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useAgentStore = create<AgentState>((set, _get) => ({
  sessions: [],
  currentSessionId: null,
  messages: [],
  streaming: false,
  streamingContent: "",
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

  // Optimistic create session (P2 - immediate UI update)
  createSession: async (novelId?: string, title?: string) => {
    const tempId = generateTempId();
    const optimisticSession: Session = {
      id: tempId,
      title: title || "New Session",
      novel_id: novelId || null,
      summary: null,
      message_count: 0,
      input_tokens: 0,
      output_tokens: 0,
      cost: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    // Optimistic update: immediately show in UI
    set((state) => ({
      sessions: [optimisticSession, ...state.sessions],
      currentSessionId: tempId,
      messages: [],
      streamingContent: "",
      streaming: false,
      error: null,
    }));

    try {
      const session = await sessionService.createSession(novelId, title);
      // Replace optimistic data with real data
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === tempId ? session : s
        ),
        currentSessionId: session.id,
      }));
      return session;
    } catch (err) {
      // Rollback optimistic update
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
    set({ currentSessionId: sessionId, loading: true, error: null, streaming: false, streamingContent: "" });
    try {
      const messages = await sessionService.listMessages(sessionId);
      set({ messages, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load messages";
      set({ error: message, loading: false });
      toast.error(message);
    }
  },

  // Optimistic delete session (P2 - immediate UI update)
  deleteSession: async (sessionId: string) => {
    // Optimistic update: immediately remove from UI
    const previousSessions = _get().sessions;
    const previousCurrentId = _get().currentSessionId;

    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== sessionId),
      currentSessionId:
        state.currentSessionId === sessionId ? null : state.currentSessionId,
      messages: state.currentSessionId === sessionId ? [] : state.messages,
    }));

    try {
      await sessionService.deleteSession(sessionId);
    } catch (err) {
      // Rollback on failure
      set({
        sessions: previousSessions,
        currentSessionId: previousCurrentId,
        error: err instanceof Error ? err.message : "Failed to delete session",
      });
      toast.error("Failed to delete session");
    }
  },

  loadMessages: async (sessionId: string) => {
    const messages = await sessionService.listMessages(sessionId);
    set({ messages });
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
      error: null,
    });
  },
}));
