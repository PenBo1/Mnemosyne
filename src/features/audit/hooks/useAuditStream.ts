// Audit & Security 实时事件订阅 hook —— 监听 security://event 推送。
//
// 设计:
// - 通过 Tauri listen() 订阅 "security://event"
// - 把新事件 prepend 到本地 events 状态
// - 提供 unreadDeniedCount:用户未确认的 denied/security 事件数
// - 卸载时自动 unlisten

import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { SecurityEventPayload } from "../types";

interface UseAuditStreamResult {
  events: SecurityEventPayload[];
  unreadDeniedCount: number;
  clearUnread: () => void;
  isConnected: boolean;
}

export function useAuditStream(enabled: boolean): UseAuditStreamResult {
  const [events, setEvents] = useState<SecurityEventPayload[]>([]);
  const [unreadDeniedCount, setUnreadDeniedCount] = useState(0);
  const [isConnected, setIsConnected] = useState(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    if (!enabled) {
      setIsConnected(false);
      return;
    }

    let cancelled = false;
    let unlisten: UnlistenFn | null = null;

    (async () => {
      try {
        unlisten = await listen<SecurityEventPayload>("security://event", (e) => {
          const payload = e.payload;
          setEvents((prev) => [payload, ...prev].slice(0, 200)); // 最多保留 200 条
          if (payload.isDenied || payload.isSecurityRelated) {
            setUnreadDeniedCount((c) => c + 1);
          }
        });
        if (cancelled) {
          unlisten();
          return;
        }
        unlistenRef.current = unlisten;
        setIsConnected(true);
      } catch (err) {
        console.error("Failed to subscribe security://event:", err);
        setIsConnected(false);
      }
    })();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
      unlistenRef.current = null;
      setIsConnected(false);
    };
  }, [enabled]);

  const clearUnread = () => setUnreadDeniedCount(0);

  return { events, unreadDeniedCount, clearUnread, isConnected };
}
