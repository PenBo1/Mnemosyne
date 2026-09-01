/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EventStream Types - 事件流类型定义
 * ═══════════════════════════════════════════════════════════════════════════
 */

// ── 事件载荷类型 ────────────────────────────────────────────────────────────

/** 基础事件载荷 */
export interface EventPayload {
  /** 事件类型 */
  type: string;
  /** 时间戳（毫秒） */
  timestamp: number;
  /** 事件数据 */
  data: unknown;
}

/** Agent 状态变更事件 */
export interface AgentStatusEvent extends EventPayload {
  type: "agent:status";
  data: {
    sessionId: string;
    status: "working" | "waiting" | "idle" | "stale" | "error" | "completed";
    message?: string;
  };
}

/** 消息创建事件 */
export interface MessageCreatedEvent extends EventPayload {
  type: "message:created";
  data: {
    sessionId: string;
    messageId: string;
    content: string;
    role: "user" | "assistant";
  };
}

/** 消息更新事件 */
export interface MessageUpdatedEvent extends EventPayload {
  type: "message:updated";
  data: {
    sessionId: string;
    messageId: string;
    delta: string;
  };
}

/** Pipeline 进度事件 */
export interface PipelineProgressEvent extends EventPayload {
  type: "pipeline:progress";
  data: {
    sessionId: string;
    phase: string;
    step: number;
    total: number;
    status: "running" | "completed" | "error";
  };
}

/** 安全事件 */
export interface SecurityEvent extends EventPayload {
  type: "security:event";
  data: {
    eventType: string;
    severity: "low" | "medium" | "high" | "critical";
    message: string;
    metadata?: Record<string, unknown>;
  };
}

// ── 订阅类型 ────────────────────────────────────────────────────────────────

/** 事件订阅配置 */
export interface EventSubscription {
  /** 订阅 ID */
  id: string;
  /** 事件类型过滤器（支持通配符） */
  eventTypes: string[];
  /** 事件处理函数 */
  handler: (event: EventPayload) => void;
  /** 是否启用防抖 */
  debounce?: number;
  /** 是否启用节流 */
  throttle?: number;
  /** 是否只触发一次 */
  once?: boolean;
}

/** 事件流配置 */
export interface EventStreamConfig {
  /** 最大重试次数 */
  maxRetries: number;
  /** 重试间隔（毫秒） */
  retryInterval: number;
  /** 熔断阈值（连续错误次数） */
  circuitBreakerThreshold: number;
  /** 熔断恢复时间（毫秒） */
  circuitBreakerResetTime: number;
  /** 事件缓冲区大小 */
  bufferSize: number;
  /** 是否启用调试日志 */
  debug: boolean;
}

/** 事件流状态 */
export interface EventStreamState {
  /** 是否已连接 */
  connected: boolean;
  /** 是否已熔断 */
  circuitOpen: boolean;
  /** 重试次数 */
  retryCount: number;
  /** 最后连接时间 */
  lastConnectedAt: number | null;
  /** 最后错误信息 */
  lastError: string | null;
  /** 接收的事件数 */
  eventCount: number;
}

// ── 默认配置 ────────────────────────────────────────────────────────────────

export const DEFAULT_EVENT_STREAM_CONFIG: EventStreamConfig = {
  maxRetries: 5,
  retryInterval: 1000,
  circuitBreakerThreshold: 5,
  circuitBreakerResetTime: 30000,
  bufferSize: 100,
  debug: false,
};