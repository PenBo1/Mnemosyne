//! 应用生命周期管理

import { useEffect, useRef, type RefObject } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { stopAllLoopRunners } from "@/features/agent/services/loop/loop-runner";

export interface LifecycleHandlers {
  onWindowClose?: () => void;
  onConfigChanged?: (config: unknown) => void;
  onError?: (error: Error) => void;
}

export interface LifecycleState {
  mounted: RefObject<boolean>;
  cleaningUp: RefObject<boolean>;
}

export function useLifecycle(handlers: LifecycleHandlers): LifecycleState {
  const mounted = useRef(true);
  const cleaningUp = useRef(false);

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];
    let cancelled = false;

    listen("window-close", () => {
      stopAllLoopRunners();
      if (handlers.onWindowClose && mounted.current) {
        handlers.onWindowClose();
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    listen("config-changed", (event) => {
      if (handlers.onConfigChanged && mounted.current) {
        handlers.onConfigChanged(event.payload);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    listen("error", (event) => {
      if (handlers.onError && mounted.current) {
        handlers.onError(event.payload as Error);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    return () => {
      cancelled = true;
      cleaningUp.current = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
      mounted.current = false;
    };
  }, []);

  return {
    mounted,
    cleaningUp,
  };
}

export async function cleanupResources(): Promise<void> {
  // 清理缓存
  const cachesKeys = await caches.keys();
  for (const cacheName of cachesKeys) {
    await caches.delete(cacheName);
  }
}