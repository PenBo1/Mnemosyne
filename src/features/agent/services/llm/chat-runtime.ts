// Chat Runtime — Rust-backed agent engine via IPC.
//
// The AI agent loop now runs in Rust (rig-core). This module is a thin IPC wrapper:
// 1. Sends user message to Rust via `chat_send_message` with a Tauri Channel
// 2. Receives streaming events (TextDelta, ToolCallStart, etc.) and updates the zustand store
// 3. Handles tool approval requests via `chat_tool_respond`
// 4. Reloads messages from DB after completion (persistence handled by Rust backend)
// 5. Monitors token usage and fires ContextCompressionCallback when threshold exceeded (P2.6)

import { invoke, Channel } from "@tauri-apps/api/core";
import { useAgentStore } from "@/features/chat/store";
import type { IpcResponse } from "@/services/ipc";
import { DEFAULT_EFFORT, isValidEffort, type EffortLevel } from "@/types/effort";
import {
  DEFAULT_COLLABORATION_STYLE,
  isValidCollaborationStyle,
  type CollaborationStyle,
} from "@/types/collaboration-style";
import type { ContextCompressionCallback } from "@/types/context-compression";
import { compactContextMessages, shouldCompact, type CompactMessage } from "../utils/compact";

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

// ── Effort Level (模块级状态,对齐 currentWorkspacePath 模式) ──
//
// EffortPicker 修改后调用 setCurrentEffort(),sendMessage 读取 currentEffort
// 持久化到 localStorage,应用启动时恢复上次选择(默认 medium)
const EFFORT_STORAGE_KEY = "mnemosyne.effortLevel";

let currentEffort: EffortLevel = (() => {
  try {
    const saved = localStorage.getItem(EFFORT_STORAGE_KEY);
    if (saved && isValidEffort(saved)) return saved;
  } catch {
    // localStorage 不可用时回退默认
  }
  return DEFAULT_EFFORT;
})();

export function getCurrentEffort(): EffortLevel {
  return currentEffort;
}

export function setCurrentEffort(effort: EffortLevel): void {
  currentEffort = effort;
  try {
    localStorage.setItem(EFFORT_STORAGE_KEY, effort);
  } catch {
    // 持久化失败不阻塞
  }
}

// ── Collaboration Style (模块级状态,对齐 currentEffort 模式) ──
//
// CollaborationStylePicker 修改后调用 setCurrentCollaborationStyle(),
// sendMessage 读取 currentCollaborationStyle
// 持久化到 localStorage,应用启动时恢复上次选择(默认 efficient)
const COLLABORATION_STYLE_STORAGE_KEY = "mnemosyne.collaborationStyle";

let currentCollaborationStyle: CollaborationStyle = (() => {
  try {
    const saved = localStorage.getItem(COLLABORATION_STYLE_STORAGE_KEY);
    if (saved && isValidCollaborationStyle(saved)) return saved;
  } catch {
    // localStorage 不可用时回退默认
  }
  return DEFAULT_COLLABORATION_STYLE;
})();

export function getCurrentCollaborationStyle(): CollaborationStyle {
  return currentCollaborationStyle;
}

export function setCurrentCollaborationStyle(style: CollaborationStyle): void {
  currentCollaborationStyle = style;
  try {
    localStorage.setItem(COLLABORATION_STYLE_STORAGE_KEY, style);
  } catch {
    // 持久化失败不阻塞
  }
}

// ── Context Compression (P2.6) ──────────────────────────────
//
// 监控 token 使用量，当 inputTokens 超过阈值时触发压缩回调。
// 前端可订阅这些事件显示压缩状态，或在 UI 上提示用户上下文过大。
//
// 注意：实际的消息历史压缩发生在 Rust 侧（agent engine 持有完整历史），
// 前端的压缩回调用于：
// 1. 通知 UI 上下文已接近/超过阈值
// 2. 对前端缓存的消息视图执行 extractive 压缩（compact.ts）用于显示

const DEFAULT_COMPRESSION_THRESHOLD = 100_000; // 100k tokens
const COMPRESSION_THRESHOLD_KEY = "mnemosyne.compressionThreshold";

let compressionThreshold: number = (() => {
  try {
    const saved = localStorage.getItem(COMPRESSION_THRESHOLD_KEY);
    if (saved) return parseInt(saved, 10);
  } catch {
    // localStorage 不可用时回退默认
  }
  return DEFAULT_COMPRESSION_THRESHOLD;
})();

let compressionCallback: ContextCompressionCallback | null = null;

/**
 * 设置压缩阈值（token 数）。当 inputTokens 超过此值时触发压缩回调。
 */
export function setCompressionThreshold(threshold: number): void {
  compressionThreshold = threshold;
  try {
    localStorage.setItem(COMPRESSION_THRESHOLD_KEY, String(threshold));
  } catch {
    // 持久化失败不阻塞
  }
}

export function getCompressionThreshold(): number {
  return compressionThreshold;
}

/**
 * 注册压缩回调。传入 null 取消注册。
 * 回调会在 token 超过阈值时被调用（phase: start → end/error）。
 */
export function setCompressionCallback(callback: ContextCompressionCallback | null): void {
  compressionCallback = callback;
}

/**
 * 对前端缓存的消息执行压缩（用于显示或预处理）。
 * 使用 compact.ts 的 compactContextMessages，默认 extractive 摘要。
 */
export async function compressMessages(
  messages: readonly CompactMessage[],
  callback?: ContextCompressionCallback,
): Promise<CompactMessage[]> {
  return compactContextMessages(messages, { callback, category: "session_context" });
}

/**
 * 检查消息列表是否需要压缩（超过阈值）。
 */
export function needsCompression(messages: readonly CompactMessage[]): boolean {
  return shouldCompact(messages, compressionThreshold);
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
  // P2.8: agent 工具白名单。undefined = 全工具集；readonly string[] = 仅这些工具。
  // 后端 SendMessageRequest 暂未声明 tool_whitelist 字段（serde 忽略未知字段），
  // 当前传递仅用于协议预声明 —— 后端加字段后即可启用 per-agent 工具限制。
  toolWhitelist?: readonly string[],
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
        // P2.6: 检查 token 是否超过压缩阈值
        if (compressionCallback && event.inputTokens > compressionThreshold) {
          compressionCallback({
            category: "session_context",
            phase: "start",
            compressibleTokens: event.inputTokens,
            budgetTokens: Math.min(
              Math.ceil(event.inputTokens * 0.2),
              12000,
            ),
            message: `Context ${event.inputTokens} tokens exceeds threshold ${compressionThreshold}`,
          });
          compressionCallback({
            category: "session_context",
            phase: "end",
            compressibleTokens: event.inputTokens,
            budgetTokens: Math.min(
              Math.ceil(event.inputTokens * 0.2),
              12000,
            ),
            message: `Context compression suggested (${event.inputTokens} tokens)`,
          });
        }
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
        effort: currentEffort,
        collaborationStyle: currentCollaborationStyle,
        // P2.8: agent 工具白名单随 IPC request 传递。后端 SendMessageRequest 暂未声明
        // tool_whitelist 字段（serde 会忽略未知字段），当前传递仅做协议预声明 ——
        // 后端补字段后即可启用 per-agent 工具限制，前端无需再改。
        toolWhitelist: toolWhitelist ? [...toolWhitelist] : null,
      },
      workspaceRoot: currentWorkspacePath,
      onEvent,
    });

    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();
    store.clearStreamingContent();
    store.clearStreamingReasoning();

    // Messages are persisted by Rust backend in engine.send_message()
    // Reload from DB to sync with persisted state
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
