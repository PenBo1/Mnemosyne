/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 全局类型导出 - 仅包含跨模块共享类型
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 领域特定类型已移动到对应的 feature 模块：
 * - book, book-rules, genre-profile → features/story/types/
 * - runtime-state, input-governance, length-governance → features/agent/types/
 */

// ── 应用全局类型 ────────────────────────────────────────────────────────

export type { AppPage, SettingsPage, AppState, FileEntry } from "./app";
export { DEFAULT_SETTINGS_PAGE, isSettingsPage } from "./app";

// ── LLM 通信类型 ────────────────────────────────────────────────────────

export type { LLMMessage, LLMResponse, StreamProgress } from "./llm";

// ── Hook 类型 ────────────────────────────────────────────────────────

export type { StoredHook, StoredSummary, Fact } from "./hook";

// ── Session 类型 ────────────────────────────────────────────────────────

export type {
  Session,
  Message,
  AgentEvent,
  PendingConfirmation,
} from "./session";

// ── 上下文压缩类型 ────────────────────────────────────────────────────────

export type {
  ContextCompressionEvent,
  ContextCompressionCallback,
} from "./context-compression";

// ── Agent 协作风格 ────────────────────────────────────────────────────────

export type { CollaborationStyle } from "./collaboration-style";

// ── Agent Effort 级别 ────────────────────────────────────────────────────────

export type { EffortLevel } from "./effort";