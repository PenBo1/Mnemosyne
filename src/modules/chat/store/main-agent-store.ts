import { create } from "zustand";
import type {
  MainAgentSession,
  MainAgentEvent,
  AgentStatus,
  ConfirmationRequest,
} from "@/shared/types/main-agent";
import * as mainAgentService from "@/features/chat/services";

interface MainAgentState {
  sessions: Record<string, MainAgentSession>;
  activeSessionId: string | null;
  loading: boolean;

  // 操作
  startExecution: (goal: string) => Promise<string>;
  respondToConfirmation: (approved: boolean, modifiedArgs?: string) => Promise<void>;
  cancelExecution: () => Promise<void>;
  setActiveSession: (id: string | null) => void;
  handleEvent: (event: MainAgentEvent) => void;
  getActiveSession: () => MainAgentSession | null;
}

export const useMainAgentStore = create<MainAgentState>((set, get) => ({
  sessions: {},
  activeSessionId: null,
  loading: false,

  startExecution: async (goal: string) => {
    const sessionId = `main-agent-${Date.now()}`;
    const now = new Date().toISOString();

    const session: MainAgentSession = {
      id: sessionId,
      goal,
      status: "Planning",
      plan: [],
      currentStep: null,
      totalSteps: null,
      messages: [
        {
          id: `msg-${Date.now()}`,
          role: "user",
          content: goal,
          timestamp: now,
        },
      ],
      confirmation: null,
      result: null,
      error: null,
      createdAt: now,
    };

    set((state) => ({
      sessions: { ...state.sessions, [sessionId]: session },
      activeSessionId: sessionId,
      loading: true,
    }));

    try {
      await mainAgentService.executeGoal({ sessionId, goal });
    } catch (e) {
      set((state) => ({
        sessions: {
          ...state.sessions,
          [sessionId]: {
            ...state.sessions[sessionId],
            status: "Failed",
            error: String(e),
          },
        },
        loading: false,
      }));
    }

    return sessionId;
  },

  respondToConfirmation: async (approved: boolean, modifiedArgs?: string) => {
    const { activeSessionId, sessions } = get();
    if (!activeSessionId) return;

    const session = sessions[activeSessionId];
    if (!session?.confirmation) return;

    // 乐观更新
    set((state) => ({
      sessions: {
        ...state.sessions,
        [activeSessionId]: {
          ...state.sessions[activeSessionId],
          confirmation: null,
          status: "Executing",
        },
      },
    }));

    try {
      await mainAgentService.respondToConfirmation({
        sessionId: activeSessionId,
        approved,
        modifiedArgs,
      });
    } catch (e) {
      console.error("Failed to respond to confirmation:", e);
    }
  },

  cancelExecution: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    try {
      await mainAgentService.cancelExecution(activeSessionId);
      set((state) => ({
        sessions: {
          ...state.sessions,
          [activeSessionId]: {
            ...state.sessions[activeSessionId],
            status: "Failed",
            error: "Cancelled by user",
          },
        },
        loading: false,
      }));
    } catch (e) {
      console.error("Failed to cancel:", e);
    }
  },

  setActiveSession: (id: string | null) => {
    set({ activeSessionId: id });
  },

  handleEvent: (event: MainAgentEvent) => {
    const { session_id } = event;

    set((state) => {
      const session = state.sessions[session_id];
      if (!session) return state;

      const updates: Partial<MainAgentSession> = {};

      switch (event.type) {
        case "Progress":
          updates.status = event.status as AgentStatus;
          updates.currentStep = event.current_step ?? null;
          updates.totalSteps = event.total_steps ?? null;
          if (event.message) {
            updates.messages = [
              ...session.messages,
              {
                id: `msg-${Date.now()}`,
                role: "system",
                content: event.message,
                timestamp: new Date().toISOString(),
              },
            ];
          }
          break;

        case "ConfirmationRequired":
          updates.status = "WaitingForConfirmation";
          updates.confirmation = {
            step_id: event.step_id!,
            description: event.description!,
            details: event.details!,
            risk_level: event.risk_level as ConfirmationRequest["risk_level"],
          };
          break;

        case "Completed":
          updates.status = "Completed";
          updates.result = event.result ?? null;
          updates.messages = [
            ...session.messages,
            {
              id: `msg-${Date.now()}`,
              role: "agent",
              content: event.result ?? "Completed",
              timestamp: new Date().toISOString(),
            },
          ];
          break;

        case "Failed":
          updates.status = "Failed";
          updates.error = event.error ?? "Unknown error";
          break;
      }

      return {
        sessions: {
          ...state.sessions,
          [session_id]: { ...session, ...updates },
        },
        loading: state.loading && event.type !== "Completed" && event.type !== "Failed",
      };
    });
  },

  getActiveSession: () => {
    const { activeSessionId, sessions } = get();
    if (!activeSessionId) return null;
    return sessions[activeSessionId] ?? null;
  },
}));
