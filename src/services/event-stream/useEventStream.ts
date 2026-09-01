/**
 * ═══════════════════════════════════════════════════════════════════════════
 * useEventStream - 事件流订阅 Hook
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useRef, useState, useCallback } from "react";
import { getEventStreamManager } from "./manager";
import type { EventPayload, EventStreamState } from "./types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface UseEventStreamOptions {
  /** 是否启用防抖（毫秒） */
  debounce?: number;
  /** 是否启用节流（毫秒） */
  throttle?: number;
  /** 是否只触发一次 */
  once?: boolean;
  /** 是否立即启用 */
  enabled?: boolean;
}

interface UseEventStreamResult<T extends EventPayload = EventPayload> {
  /** 接收到的事件列表 */
  events: T[];
  /** 当前连接状态 */
  state: EventStreamState;
  /** 清空事件列表 */
  clear: () => void;
  /** 手动重新连接 */
  reconnect: () => void;
}

// ── 主 Hook ──────────────────────────────────────────────────────────────────

/**
 * 订阅事件流的 React Hook
 * @param eventType 事件类型（支持通配符 "agent:*" 或 "*"）
 * @param handler 事件处理函数（可选）
 * @param options 订阅选项
 */
export function useEventStream<T extends EventPayload = EventPayload>(
  eventType: string,
  handler?: (event: T) => void,
  options: UseEventStreamOptions = {}
): UseEventStreamResult<T> {
  const { debounce, throttle, once, enabled = true } = options;
  const [events, setEvents] = useState<T[]>([]);
  const [state, setState] = useState<EventStreamState>({
    connected: false,
    circuitOpen: false,
    retryCount: 0,
    lastConnectedAt: null,
    lastError: null,
    eventCount: 0,
  });

  const manager = getEventStreamManager();
  const unsubscribeRef = useRef<(() => void) | null>(null);

  // 事件处理：收集事件并调用外部 handler
  const handleEvent = useCallback(
    (event: EventPayload) => {
      const typedEvent = event as T;
      setEvents((prev) => [typedEvent, ...prev]);
      if (handler) {
        handler(typedEvent);
      }
    },
    [handler]
  );

  useEffect(() => {
    if (!enabled) {
      // 禁用时清理订阅
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
        unsubscribeRef.current = null;
      }
      setState((prev) => ({ ...prev, connected: false }));
      return;
    }

    // 订阅事件
    unsubscribeRef.current = manager.subscribe(eventType, handleEvent, {
      debounce,
      throttle,
      once,
    });

    // 同步状态
    setState(manager.getState());

    return () => {
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
        unsubscribeRef.current = null;
      }
    };
  }, [eventType, enabled, debounce, throttle, once, manager, handleEvent]);

  // 定期同步状态
  useEffect(() => {
    if (!enabled) return;

    const interval = setInterval(() => {
      setState(manager.getState());
    }, 1000);

    return () => clearInterval(interval);
  }, [enabled, manager]);

  const clear = useCallback(() => {
    setEvents([]);
    manager.clearBuffer();
  }, [manager]);

  const reconnect = useCallback(() => {
    manager.disconnect().then(() => {
      setState(manager.getState());
    });
  }, [manager]);

  return { events, state, clear, reconnect };
}

// ── 专用 Hooks ──────────────────────────────────────────────────────────────

/**
 * 订阅 Agent 状态变更事件
 */
export function useAgentStatusEvent(
  handler?: (event: EventPayload) => void,
  options?: UseEventStreamOptions
) {
  return useEventStream("agent:status", handler, options);
}

/**
 * 订阅消息创建事件
 */
export function useMessageCreatedEvent(
  handler?: (event: EventPayload) => void,
  options?: UseEventStreamOptions
) {
  return useEventStream("message:created", handler, options);
}

/**
 * 订阅 Pipeline 进度事件
 */
export function usePipelineProgressEvent(
  handler?: (event: EventPayload) => void,
  options?: UseEventStreamOptions
) {
  return useEventStream("pipeline:progress", handler, options);
}

/**
 * 订阅所有安全相关事件
 */
export function useSecurityEvent(
  handler?: (event: EventPayload) => void,
  options?: UseEventStreamOptions
) {
  return useEventStream("security:*", handler, options);
}