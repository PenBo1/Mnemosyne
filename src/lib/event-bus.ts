import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** 事件通道注册表：集中管理 Tauri 事件名，避免散落字符串字面量。 */
export const EventChannels = {
  /** Agent 流式事件：TurnStarted/StreamDelta/ToolCallBegin/ToolCallEnd/TurnCompleted/Error/CompactionTriggered */
  AgentEvent: "agent-event",
} as const;

export type EventChannelName = (typeof EventChannels)[keyof typeof EventChannels];

async function subscribe<T>(
  channel: EventChannelName,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(channel, (event) => handler(event.payload));
}

/**
 * 事件订阅 hook：组件挂载时订阅，卸载时取消；防竞态（异步 listen 完成前
 * 组件已卸载时立即清理）。通过 handlerRef 保持 handler 引用最新。
 */
export function useEventSubscription<T>(
  channel: EventChannelName,
  handler: (payload: T) => void,
  enabled = true,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!enabled) return;

    let cancelled = false;
    let unlisten: UnlistenFn | null = null;

    subscribe<T>(channel, (payload) => {
      if (!cancelled) handlerRef.current(payload);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [channel, enabled]);
}
