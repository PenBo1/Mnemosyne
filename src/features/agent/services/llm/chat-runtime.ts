/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Chat Runtime - Rust-backed agent engine via IPC
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * The AI agent loop now runs in Rust (rig-core). This module is a thin IPC wrapper:
 * 1. Sends user message to Rust via `chat_send_message` with a Tauri Channel
 * 2. Receives streaming events (TextDelta, ToolCallStart, etc.) and updates the zustand store
 * 3. Handles tool approval requests via `chat_tool_respond`
 * 4. Reloads messages from DB after completion (persistence handled by Rust backend)
 * 5. Monitors token usage and fires ContextCompressionCallback when threshold exceeded (P2.6)
 */

import { invoke, Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";
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
import {
  FailureDetector,
  type FailureReport,
  type FailureCallback,
  type AgentStep,
  type ToolRegistry,
} from "../failure-detection";

// ── Module-level state safety limits ───────────────────────
//
// 防止模块级单例在长时间运行或会话切换时泄漏资源：
// - MAX_TRACE_STEPS: currentTraceSteps 环形缓冲上限（超出时丢弃最旧 step）
// - CHANNEL_TIMEOUT_MS: 后端无响应时强制 flush + setStreaming(false) 的阈值

const MAX_TRACE_STEPS = 1000;
const CHANNEL_TIMEOUT_MS = 5 * 60 * 1000; // 5 分钟

// 上下文压缩建议的预算计算参数（finish 事件触发压缩回调时使用）
const CONTEXT_COMPRESS_RATIO = 0.2; // 保留比例：原上下文的 20%
const MAX_CONTEXT_TOKENS = 12000; // 压缩后预算上限

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

interface ToolCallDeltaEvent extends ChatEventBase {
  kind: "toolCallDelta";
  id: string;
  argsDelta: string;
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

interface RetryEvent extends ChatEventBase {
  kind: "retry";
  attempt: number;
  maxAttempts: number;
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
  | ToolCallDeltaEvent
  | ToolCallEndEvent
  | ToolApprovalRequiredEvent
  | RetryEvent
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

// ── Failure Detection ───────────────────────────────────────
//
// 失败模式检测器用于检测 Agent 执行过程中的异常模式：
// - HallucinatedAction: 调用不存在的工具
// - ScopeCreep: 操作范围超出原始请求
// - CascadingError: 错误级联传播
// - ContextLoss: 约束遗忘
// - ToolMisuse: 工具参数错误
//
// 检测器以非阻塞方式运行，不会停止 Agent 执行。

let failureDetector: FailureDetector | null = null;
let failureCallbacks: Set<FailureCallback> = new Set();
let currentTraceSteps: AgentStep[] = [];
let currentOriginalRequest: string | null = null;
let currentConstraints: string[] = [];

export function initFailureDetector(toolRegistry?: ToolRegistry): void {
  failureDetector = new FailureDetector({ toolRegistry });
}

export function getFailureDetector(): FailureDetector | null {
  return failureDetector;
}

export function onFailureDetected(callback: FailureCallback): () => void {
  failureCallbacks.add(callback);
  return () => {
    failureCallbacks.delete(callback);
  };
}

export function setOriginalRequest(request: string): void {
  currentOriginalRequest = request;
  currentTraceSteps = [];
  if (failureDetector) {
    failureDetector.setOriginalRequest(request);
  }
}

export function setConstraints(constraints: string[]): void {
  currentConstraints = constraints;
  if (failureDetector) {
    failureDetector.setConstraints(constraints);
  }
}

function emitFailure(report: FailureReport): void {
  for (const callback of failureCallbacks) {
    try {
      callback(report);
    } catch (error) {
      console.error("[chat-runtime] Failure callback error:", error);
    }
  }
}

function processToolCallForFailureDetection(
  id: string,
  name: string,
  status: "pending" | "running" | "succeeded" | "failed",
  error?: string,
  args?: Record<string, unknown>,
  result?: unknown,
): void {
  if (!failureDetector) return;

  const step: AgentStep = {
    id,
    type: "tool_call",
    toolName: name,
    toolArgs: args,
    toolResult: result,
    status,
    error,
    timestamp: Date.now(),
  };

  // 环形缓冲：超过上限时丢弃最旧 step，防止长时间运行内存泄漏
  if (currentTraceSteps.length >= MAX_TRACE_STEPS) {
    currentTraceSteps.shift();
  }
  currentTraceSteps.push(step);

  const reports = failureDetector.onStepFinish(step);
  for (const report of reports) {
    emitFailure(report);
  }
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
  // 会话切换守卫：取消上一次会话遗留的 rafId 并 flush 残留 buffer，
  // 防止 pending rAF 回调将旧 session 内容写入新 session 的 store
  flushNow();
  const store = useAgentStore.getState();
  store.clearStreamingContent();
  store.clearStreamingReasoning();
  store.clearActiveToolCalls();

  // 初始化失败检测器
  if (!failureDetector) {
    initFailureDetector();
  }
  setOriginalRequest(content);

  const onEvent = new Channel<ChatEvent>();

  onEvent.onmessage = (event: ChatEvent) => {
    console.log("[chat-runtime] event received", event.kind);
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
        store.addToolCallStart({ id: event.id, name: event.name, startPosition: textBuffer.length });
        // 失败检测：工具调用开始（pending 状态）
        processToolCallForFailureDetection(event.id, event.name, "pending");
        break;
      case "toolCallDelta":
        // 累积工具调用参数（流式），更新 store 中的工具调用状态
        store.appendToolCallArgs(event.id, event.argsDelta);
        break;
      case "toolCallEnd":
        store.updateToolCallEnd(event.id);
        // 失败检测：工具调用结束（succeeded 状态）
        processToolCallForFailureDetection(event.id, "", "succeeded");
        break;
      case "toolApprovalRequired":
        store.setPendingConfirmation({
          toolCallId: event.requestId,
          toolName: event.name,
          args: event.args,
        });
        break;
      case "retry":
        // 重试事件：LLM 流式调用因临时性错误（503/429 等）重试，仅日志不阻塞主流程
        console.log(
          `[chat-runtime] Retrying (attempt ${event.attempt}/${event.maxAttempts})`,
        );
        break;
      case "finish":
        flushNow();
        // 确保流式状态正确结束
        store.setStreaming(false);
        // P2.6: 检查 token 是否超过压缩阈值
        if (compressionCallback && event.inputTokens > compressionThreshold) {
          compressionCallback({
            category: "session_context",
            phase: "start",
            compressibleTokens: event.inputTokens,
            budgetTokens: Math.min(
              Math.ceil(event.inputTokens * CONTEXT_COMPRESS_RATIO),
              MAX_CONTEXT_TOKENS,
            ),
            message: `Context ${event.inputTokens} tokens exceeds threshold ${compressionThreshold}`,
          });
          compressionCallback({
            category: "session_context",
            phase: "end",
            compressibleTokens: event.inputTokens,
            budgetTokens: Math.min(
              Math.ceil(event.inputTokens * CONTEXT_COMPRESS_RATIO),
              MAX_CONTEXT_TOKENS,
            ),
            message: `Context compression suggested (${event.inputTokens} tokens)`,
          });
        }
        // 失败检测：完成后分析完整 trace
        if (failureDetector && currentTraceSteps.length > 0) {
          const traceReports = failureDetector.analyzeTrace({
            steps: currentTraceSteps,
            originalRequest: currentOriginalRequest ?? undefined,
            constraints: currentConstraints.length > 0 ? currentConstraints : undefined,
          });
          for (const report of traceReports) {
            emitFailure(report);
          }
        }
        break;
      case "error":
        flushNow();
        store.setStreaming(false);
        store.clearActiveToolCalls();
        store.setError(event.message);
        toast.error(event.message);
        // 失败检测：错误事件
        if (failureDetector) {
          const errorStep: AgentStep = {
            id: `error-${Date.now()}`,
            type: "tool_call",
            status: "failed",
            error: event.message,
            timestamp: Date.now(),
          };
          const reports = failureDetector.onStepFinish(errorStep);
          for (const report of reports) {
            emitFailure(report);
          }
        }
        break;
    }
  };

  // Channel 超时守卫：后端若 5 分钟内未完成（卡死/网络分区），强制 flush 并
  // 结束流式状态，避免 await invoke 永久挂起。不抛错，让上层按"已结束"处理。
  const channelTimeoutId = setTimeout(() => {
    console.warn(
      `[chat-runtime] Channel no completion within ${CHANNEL_TIMEOUT_MS}ms, force flushing`,
    );
    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();
  }, CHANNEL_TIMEOUT_MS);

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

    console.log("[chat-runtime] invoke completed successfully");
    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();
    store.clearStreamingContent();
    store.clearStreamingReasoning();

    // Messages are persisted by Rust backend in engine.send_message()
    // Reload from DB to sync with persisted state
    await store.loadMessages(sessionId);
  } catch (err) {
    console.error("[chat-runtime] invoke failed", err);
    flushNow();
    store.setStreaming(false);
    store.clearActiveToolCalls();
    const msg = err instanceof Error ? err.message : "Failed to send message";
    store.setError(msg);
    toast.error(msg);
    throw err;
  } finally {
    clearTimeout(channelTimeoutId);
  }
}

// ── Stop chat ───────────────────────────────────────────────

export function stopChat(sessionId: string): void {
  invoke("chat_stop", { sessionId }).catch((err) => {
    console.error("[chat-runtime] stopChat failed", err);
  });
  flushNow();
  const store = useAgentStore.getState();
  store.setStreaming(false);
  store.clearActiveToolCalls();
  // 清空失败检测回调集合，防止模块级 Set 在多次 stop 后累积未释放的引用
  failureCallbacks.clear();
}
