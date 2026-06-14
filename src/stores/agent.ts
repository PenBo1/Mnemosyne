import { create } from "zustand";
import type { Session, Message } from "@/types";
import * as sessionService from "@/services/session";

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
    }
  },

  createSession: async (novelId?: string, title?: string) => {
    const session = await sessionService.createSession(novelId, title);
    set((state) => ({
      sessions: [session, ...state.sessions],
      currentSessionId: session.id,
      messages: [],
      streamingContent: "",
      streaming: false,
      error: null,
    }));
    return session;
  },

  switchSession: async (sessionId: string) => {
    set({ currentSessionId: sessionId, loading: true, error: null, streaming: false, streamingContent: "" });
    try {
      const messages = await sessionService.listMessages(sessionId);
      set({ messages, loading: false });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load messages";
      set({ error: message, loading: false });
    }
  },

  deleteSession: async (sessionId: string) => {
    await sessionService.deleteSession(sessionId);
    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== sessionId),
      currentSessionId:
        state.currentSessionId === sessionId ? null : state.currentSessionId,
      messages: state.currentSessionId === sessionId ? [] : state.messages,
    }));
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
