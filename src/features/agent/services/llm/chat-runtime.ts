// Chat Runtime — Rust-backed agent engine via IPC.
//
// The AI agent loop now runs in Rust (rig-core). This module is a thin IPC wrapper:
// 1. Sends user message to Rust via `chat_send_message` with a Tauri Channel
// 2. Receives streaming events (TextDelta, ToolCallStart, etc.) and updates the zustand store
// 3. Handles tool approval requests via `chat_tool_respond`
// 4. Persists messages to DB after completion

import { invoke, Channel } from "@tauri-apps/api/core";
import { useAgentStore } from "@/features/chat/store";
import { createMessage } from "@/features/session/services";
import type { IpcResponse } from "@/services/ipc";

// ── Chat Event Types (mirrors Rust ChatEvent enum) ───────────

interface ChatEventBase {
  kind: string;
}

interface TextDeltaEvent extends ChatEventBase {
  kind: "textDelta";
  content: string;
}

interface ReasoningDeltaEvent extends ChatEventBase {
  kind: "reasoningDelta";
  content: string;
}

interface ToolCallStartEvent extends ChatEventBase {
  kind: "toolCallStart";
  id: string;
  name: string;
}

interface ToolCallEndEvent extends ChatEventBase {
  kind: "toolCallEnd";
  id: string;
}

interface ToolApprovalRequiredEvent extends ChatEventBase {
  kind: "toolApprovalRequired";
  requestId: string;
  name: string;
  args: Record<string, unknown>;
}

interface FinishEvent extends ChatEventBase {
  kind: "finish";
  inputTokens: number;
  outputTokens: number;
}

interface ErrorEvent extends ChatEventBase {
  kind: "error";
  message: string;
}

type ChatEvent =
  | TextDeltaEvent
  | ReasoningDeltaEvent
  | ToolCallStartEvent
  | ToolCallEndEvent
  | ToolApprovalRequiredEvent
  | FinishEvent
  | ErrorEvent;

// ── Streaming buffers (rAF batched) ─────────────────────────

let textBuffer = "";
let reasoningBuffer = "";
let rafId: number | null = null;

function flushStreamBuffers(): void {
  rafId = null;
  const store = useAgentStore.getState();
  if (textBuffer) {
    store.updateStreamingContent(textBuffer);
    textBuffer = "";
  }
  if (reasoningBuffer) {
    store.updateStreamingReasoning(reasoningBuffer);
    reasoningBuffer = "";
  }
}

function scheduleFlush(): void {
  if (rafId === null) {
    rafId = requestAnimationFrame(flushStreamBuffers);
  }
}

function flushNow(): void {
  if (rafId !== null) {
    cancelAnimationFrame(rafId);
    rafId = null;
  }
  const store = useAgentStore.getState();
  if (textBuffer) {
    store.updateStreamingContent(textBuffer);
    textBuffer = "";
  }
  if (reasoningBuffer) {
    store.updateStreamingReasoning(reasoningBuffer);
    reasoningBuffer = "";
  }
}

// ── Session context ─────────────────────────────────────────

let currentWorkspacePath: string | null = null;

export function setCurrentWorkspacePath(path: string | null): void {
  currentWorkspacePath = path;
}

// Legacy exports for backward compatibility during migration
export function setCurrentModelConfig(_config: unknown): void {
  // No-op: model config is managed by Rust backend now
}

export function setCurrentCustomInstructions(_text: string): void {
  // No-op: custom instructions passed via IPC request
}

// ── Send message ────────────────────────────────────────────

export async function sendMessage(
  sessionId: string,
  content: string,
  contextText?: string,
  customInstructions?: string,
): Promise<void> {
  const store = useAgentStore.getState();
  store.clearStreamingContent();
  store.clearStreamingReasoning();
  store.clearActiveToolCalls();

  const onEvent = new Channel<ChatEvent>();

  onEvent.onmessage = (event: ChatEvent) => {
    switch (event.kind) {
      case "textDelta":
        textBuffer += event.content;
        scheduleFlush();
        break;
      case "reasoningDelta":
        reasoningBuffer += event.content;
        scheduleFlush();
        break;
      case "toolCallStart":
        store.addToolCallStart({ id: event.id, name: event.name });
        break;
      case "toolCallEnd":
        store.updateToolCallEnd(event.id);
        break;
      case "toolApprovalRequired":
        store.setPendingConfirmation({
          toolCallId: event.requestId,
          toolName: event.name,
          args: event.args,
        });
        break;
      case "finish":
        flushNow();
        break;
      case "error":
        flushNow();
        store.setStreaming(false);
        store.clearActiveToolCalls();
        store.setError(event.message);
        break;
    }
  };

  try {
    await invoke<IpcResponse<void>>("chat_send_message", {
      request: {
        sessionId,
        content,
        contextText: contextText ?? null,
        customInstructions: customInstructions ?? null,
      },
      workspaceRoot: currentWorkspacePath,
      onEvent,
    });

    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();

    // Persist messages to DB
    await createMessage(sessionId, "user", content);
    const fullContent = useAgentStore.getState().streamingContent;
    if (fullContent) {
      await createMessage(sessionId, "assistant", fullContent);
    }

    // Reload messages from DB
    await store.loadMessages(sessionId);
  } catch (err) {
    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();
    const msg = err instanceof Error ? err.message : "Failed to send message";
    store.setError(msg);
    throw err;
  }
}

// ── Stop chat ───────────────────────────────────────────────

export function stopChat(sessionId: string): void {
  invoke("chat_stop", { sessionId }).catch(() => {
    // Ignore errors on stop
  });
  flushNow();
  const store = useAgentStore.getState();
  store.setStreaming(false);
  store.clearActiveToolCalls();
}
