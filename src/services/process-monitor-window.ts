/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 进程监控窗口服务 - 提供进程监控窗口的创建与管理功能
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Event } from "@tauri-apps/api/event";

// ── 常量定义 ────────────────────────────────────────────────────────────────

const WINDOW_LABEL = "process-monitor";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

// 检测是否为开发模式
const isDev = import.meta.env.DEV;

// 获取正确的 URL
const getProcessMonitorUrl = (): string => {
  if (isDev) {
    // 开发模式：使用 Vite 开发服务器的路径
    return "http://localhost:1420/src/process-monitor/index.html";
  }
  // 生产模式：使用构建后的路径
  return "/src/process-monitor/index.html";
};

// ── 窗口操作 ────────────────────────────────────────────────────────────────

/**
 * 打开进程监控窗口
 *
 * 如果窗口已存在，则聚焦该窗口；否则创建新窗口。
 */
export async function openProcessMonitorWindow(): Promise<void> {
  // 检查窗口是否已存在
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  if (existing) {
    // 窗口已存在，聚焦它
    await existing.setFocus();
    return;
  }

  // 创建新窗口
  const webview = new WebviewWindow(WINDOW_LABEL, {
    url: getProcessMonitorUrl(),
    title: "Process Monitor - Mnemosyne",
    width: 650,
    height: 450,
    minWidth: 400,
    minHeight: 300,
    resizable: true,
    center: true,
    decorations: true,
    transparent: false,
    alwaysOnTop: false,
  });

  // 监听窗口创建错误
  webview.once("tauri://error", (e: Event<unknown>) => {
    console.error("Failed to create process monitor window:", e);
  });
}

/**
 * 关闭进程监控窗口（如果存在）
 */
export async function closeProcessMonitorWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  if (existing) {
    await existing.close();
  }
}

/**
 * 检查进程监控窗口是否已打开
 */
export async function isProcessMonitorWindowOpen(): Promise<boolean> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  return existing !== null;
}