/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EventStreamManager - 事件流管理器
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  EventPayload,
  EventSubscription,
  EventStreamConfig,
  EventStreamState,
} from "./types";
import { DEFAULT_EVENT_STREAM_CONFIG } from "./types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type EventHandler = (event: EventPayload) => void;
type MatcherFunction = (eventType: string) => boolean;

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 创建事件类型匹配器
 * 支持 "*" 通配符匹配所有事件
 * 支持 "prefix:*" 匹配特定前缀的事件
 */
function createMatcher(pattern: string): MatcherFunction {
  if (pattern === "*") {
    return () => true;
  }
  if (pattern.endsWith(":*")) {
    const prefix = pattern.slice(0, -1);
    return (eventType: string) => eventType.startsWith(prefix);
  }
  return (eventType: string) => eventType === pattern;
}

/**
 * 创建防抖函数
 */
function debounce(fn: EventHandler, delay: number): EventHandler {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (event: EventPayload) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => fn(event), delay);
  };
}

/**
 * 创建节流函数
 */
function throttle(fn: EventHandler, limit: number): EventHandler {
  let inThrottle = false;
  return (event: EventPayload) => {
    if (!inThrottle) {
      fn(event);
      inThrottle = true;
      setTimeout(() => {
        inThrottle = false;
      }, limit);
    }
  };
}

// ── 主类 ────────────────────────────────────────────────────────────────────

/**
 * 事件流管理器
 * 统一管理 Tauri 事件订阅，支持熔断、重连、过滤
 */
export class EventStreamManager {
  private config: EventStreamConfig;
  private subscriptions: Map<string, EventSubscription> = new Map();
  private unlisteners: Map<string, UnlistenFn> = new Map();
  private state: EventStreamState;
  private errorCount: number = 0;
  private circuitOpenedAt: number | null = null;
  private eventBuffer: EventPayload[] = [];

  constructor(config: Partial<EventStreamConfig> = {}) {
    this.config = { ...DEFAULT_EVENT_STREAM_CONFIG, ...config } as EventStreamConfig;
    this.state = {
      connected: false,
      circuitOpen: false,
      retryCount: 0,
      lastConnectedAt: null,
      lastError: null,
      eventCount: 0,
    };
  }

  /**
   * 获取当前状态
   */
  getState(): EventStreamState {
    return { ...this.state };
  }

  /**
   * 订阅事件
   * @param eventType 事件类型（支持通配符）
   * @param handler 事件处理函数
   * @param options 订阅选项
   * @returns 取消订阅函数
   */
  subscribe(
    eventType: string,
    handler: EventHandler,
    options: {
      debounce?: number;
      throttle?: number;
      once?: boolean;
    } = {}
  ): () => void {
    const id = `${eventType}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

    // 应用防抖/节流
    let processedHandler = handler;
    if (options.debounce) {
      processedHandler = debounce(handler, options.debounce);
    } else if (options.throttle) {
      processedHandler = throttle(handler, options.throttle);
    }

    // 如果是一次性订阅，包装处理函数
    if (options.once) {
      const originalHandler = processedHandler;
      processedHandler = (event: EventPayload) => {
        originalHandler(event);
        this.unsubscribe(id);
      };
    }

    const subscription: EventSubscription = {
      id,
      eventTypes: [eventType],
      handler: processedHandler,
      ...options,
    };

    this.subscriptions.set(id, subscription);
    this.ensureListener(eventType);

    return () => this.unsubscribe(id);
  }

  /**
   * 取消订阅
   */
  unsubscribe(id: string): void {
    this.subscriptions.delete(id);
    this.cleanupUnusedListeners();
  }

  /**
   * 订阅多个事件类型
   */
  subscribeMultiple(
    eventTypes: string[],
    handler: EventHandler,
    options: { debounce?: number; throttle?: number } = {}
  ): () => void {
    const id = `multi-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

    let processedHandler = handler;
    if (options.debounce) {
      processedHandler = debounce(handler, options.debounce);
    } else if (options.throttle) {
      processedHandler = throttle(handler, options.throttle);
    }

    const subscription: EventSubscription = {
      id,
      eventTypes,
      handler: processedHandler,
      ...options,
    };

    this.subscriptions.set(id, subscription);
    eventTypes.forEach((type) => this.ensureListener(type));

    return () => this.unsubscribe(id);
  }

  /**
   * 获取事件缓冲区
   */
  getBuffer(): EventPayload[] {
    return [...this.eventBuffer];
  }

  /**
   * 清空事件缓冲区
   */
  clearBuffer(): void {
    this.eventBuffer = [];
  }

  /**
   * 关闭所有连接
   */
  async disconnect(): Promise<void> {
    for (const [eventType, unlisten] of this.unlisteners) {
      unlisten();
      this.log(`Unlistened: ${eventType}`);
    }
    this.unlisteners.clear();
    this.state.connected = false;
  }

  // ── 私有方法 ──────────────────────────────────────────────────────────────

  /**
   * 确保事件监听器已设置
   */
  private async ensureListener(eventType: string): Promise<void> {
    // 检查是否需要监听新的事件类型
    const pattern = this.getPatternForEventType(eventType);
    if (!pattern || this.unlisteners.has(pattern)) {
      return;
    }

    // 检查熔断状态
    if (this.isCircuitOpen()) {
      this.log(`Circuit open, skipping listener for: ${eventType}`);
      return;
    }

    try {
      const unlisten = await listen<EventPayload>(pattern, (event) => {
        this.handleEvent(event.payload);
      });

      this.unlisteners.set(pattern, unlisten);
      this.state.connected = true;
      this.state.lastConnectedAt = Date.now();
      this.errorCount = 0;
      this.state.retryCount = 0;

      this.log(`Listening: ${pattern}`);
    } catch (err) {
      this.handleError(err as Error, eventType);
    }
  }

  /**
   * 获取事件类型对应的监听模式
   */
  private getPatternForEventType(eventType: string): string {
    // 如果是通配符，直接返回
    if (eventType === "*") return "*";
    if (eventType.endsWith(":*")) return eventType;

    // 否则监听具体事件
    return eventType;
  }

  /**
   * 处理接收到的事件
   */
  private handleEvent(payload: EventPayload): void {
    this.state.eventCount++;

    // 添加到缓冲区
    this.eventBuffer.push(payload);
    if (this.eventBuffer.length > this.config.bufferSize) {
      this.eventBuffer.shift();
    }

    // 分发给订阅者
    for (const subscription of this.subscriptions.values()) {
      if (this.matchesEventTypes(payload.type, subscription.eventTypes)) {
        try {
          subscription.handler(payload);
        } catch (err) {
          this.log(`Handler error for ${subscription.id}:`, err);
        }
      }
    }
  }

  /**
   * 检查事件类型是否匹配
   */
  private matchesEventTypes(eventType: string, patterns: string[]): boolean {
    return patterns.some((pattern) => {
      const matcher = createMatcher(pattern);
      return matcher(eventType);
    });
  }

  /**
   * 处理错误
   */
  private handleError(error: Error, context: string): void {
    this.errorCount++;
    this.state.lastError = error.message;
    this.log(`Error in ${context}:`, error.message);

    // 检查是否需要熔断
    if (this.errorCount >= this.config.circuitBreakerThreshold) {
      this.openCircuit();
    }
  }

  /**
   * 检查熔断器是否打开
   */
  private isCircuitOpen(): boolean {
    if (!this.state.circuitOpen) return false;

    // 检查是否可以尝试恢复
    if (this.circuitOpenedAt) {
      const elapsed = Date.now() - this.circuitOpenedAt;
      if (elapsed >= this.config.circuitBreakerResetTime) {
        this.closeCircuit();
        return false;
      }
    }

    return true;
  }

  /**
   * 打开熔断器
   */
  private openCircuit(): void {
    this.state.circuitOpen = true;
    this.circuitOpenedAt = Date.now();
    this.log("Circuit breaker opened");
  }

  /**
   * 关闭熔断器
   */
  private closeCircuit(): void {
    this.state.circuitOpen = false;
    this.circuitOpenedAt = null;
    this.errorCount = 0;
    this.log("Circuit breaker closed");
  }

  /**
   * 清理未使用的监听器
   */
  private cleanupUnusedListeners(): void {
    const activePatterns = new Set<string>();
    for (const sub of this.subscriptions.values()) {
      for (const type of sub.eventTypes) {
        activePatterns.add(this.getPatternForEventType(type));
      }
    }

    for (const [pattern, unlisten] of this.unlisteners) {
      if (!activePatterns.has(pattern)) {
        unlisten();
        this.unlisteners.delete(pattern);
        this.log(`Cleaned up listener: ${pattern}`);
      }
    }
  }

  /**
   * 日志输出
   */
  private log(...args: unknown[]): void {
    if (this.config.debug) {
      console.log("[EventStream]", ...args);
    }
  }
}

// ── 单例实例 ────────────────────────────────────────────────────────────────

let instance: EventStreamManager | null = null;

/**
 * 获取全局事件流管理器实例
 */
export function getEventStreamManager(
  config?: Partial<EventStreamConfig>
): EventStreamManager {
  if (!instance) {
    instance = new EventStreamManager(config);
  }
  return instance;
}