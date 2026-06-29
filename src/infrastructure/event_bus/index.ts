import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { EventChannels, type EventChannelName } from "./channels";

/**
 * 订阅 Tauri 事件。
 *
 * 统一替代此前散落在 services/ 各处的 `listen()` 直接调用。
 * 返回取消订阅函数，调用方负责在合适的时机释放。
 */
async function subscribe<T>(
  channel: EventChannelName,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(channel, (event) => handler(event.payload));
}

/**
 * 事件订阅 hook。
 *
 * 自动管理生命周期：组件挂载时订阅，卸载时取消订阅；防竞态（异步 listen
 * 完成前组件已卸载时立即清理，避免泄漏）。
 *
 * 通过 handlerRef 保持 handler 引用最新，无需调用方依赖 useCallback。
 *
 * 替代此前每个 hook 各自手写 useEffect + listen + unlisten 的样板。
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

export { EventChannels };
