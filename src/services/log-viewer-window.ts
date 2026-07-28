/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 日志查看窗口服务 - 提供日志查看窗口的创建与管理功能
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Event } from "@tauri-apps/api/event";

// ── 常量定义 ────────────────────────────────────────────────────────────────

const WINDOW_LABEL = "log-viewer";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

// 检测是否为开发模式
const isDev = import.meta.env.DEV;

// 获取正确的 URL
const getLogViewerUrl = (): string => {
  if (isDev) {
    return "http://localhost:1420/src/log-viewer/index.html";
  }
  return "/src/log-viewer/index.html";
};

// ── 窗口操作 ────────────────────────────────────────────────────────────────

/**
 * 打开日志查看窗口
 *
 * 如果窗口已存在，则聚焦该窗口；否则创建新窗口。
 */
export async function openLogViewerWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  if (existing) {
    await existing.setFocus();
    return;
  }

  const webview = new WebviewWindow(WINDOW_LABEL, {
    url: getLogViewerUrl(),
    title: "Log Viewer - Mnemosyne",
    width: 800,
    height: 500,
    minWidth: 400,
    minHeight: 300,
    resizable: true,
    center: true,
    decorations: true,
    transparent: false,
    alwaysOnTop: false,
  });

  webview.once("tauri://error", (e: Event<unknown>) => {
    console.error("Failed to create log viewer window:", e);
  });
}

/**
 * 关闭日志查看窗口（如果存在）
 */
export async function closeLogViewerWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  if (existing) {
    await existing.close();
  }
}

/**
 * 检查日志查看窗口是否已打开
 */
export async function isLogViewerWindowOpen(): Promise<boolean> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  return existing !== null;
}