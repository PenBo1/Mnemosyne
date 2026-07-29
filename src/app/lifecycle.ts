//! 应用生命周期管理

import { useEffect, useRef, type RefObject } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

    // 监听窗口关闭请求事件（从 Rust on_window_event 触发）
    listen("close-requested", async () => {
      // 动态导入 store 检查 agent 是否正在运行
      const { useAgentStore } = await import("@/features/chat/store");
      const agentStore = useAgentStore.getState();

      if (agentStore.isAgentRunning()) {
        // Agent 正在运行，显示确认对话框
        const { ask } = await import("@tauri-apps/plugin-dialog");
        const confirmed = await ask(
          "Agent is currently running a task. Interrupting will lose current progress. Exit anyway?",
          {
            title: "Agent is Running",
            kind: "warning",
            okLabel: "Exit Anyway",
            cancelLabel: "Cancel",
          }
        );

        if (confirmed) {
          // 用户确认退出
          agentStore.forceStop();
          stopAllLoopRunners();
          if (handlers.onWindowClose && mounted.current) {
            handlers.onWindowClose();
          }
          // 通知 Rust 允许关闭
          const mainWindow = getCurrentWindow();
          await mainWindow.emit("allow-close");
        }
        // 如果用户取消，不做任何事（窗口保持打开）
      } else {
        // Agent 未运行，直接关闭
        stopAllLoopRunners();
        if (handlers.onWindowClose && mounted.current) {
          handlers.onWindowClose();
        }
        // 通知 Rust 允许关闭
        const mainWindow = getCurrentWindow();
        await mainWindow.emit("allow-close");
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

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