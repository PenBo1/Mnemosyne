// Context Compression 类型 —— 纯类型定义。
//
// 上下文压缩回调事件类型，供压缩进度通知用。
// 极简类型（15 行），完全独立无外部依赖。

export type ContextCompressionCategory = "session_context" | "story_context";
export type ContextCompressionPhase = "start" | "end" | "error";

export interface ContextCompressionEvent {
  readonly category: ContextCompressionCategory;
  readonly phase: ContextCompressionPhase;
  readonly message?: string;
  readonly protectedTokens?: number;
  readonly compressibleTokens?: number;
  readonly budgetTokens?: number;
  readonly sources?: readonly string[];
}

export type ContextCompressionCallback = (event: ContextCompressionEvent) => void;