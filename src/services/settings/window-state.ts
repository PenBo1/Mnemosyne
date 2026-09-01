/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 窗口状态管理服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { WindowBounds } from "./types";
import { loadSettings, saveSettings } from "./settings-core";

// ── 日志级别 ────────────────────────────────────────────────────────────────

export async function getLogLevel(): Promise<"trace" | "debug" | "info" | "warn" | "error"> {
  const settings = await loadSettings();
  return settings.system?.log_level ?? "info";
}

export async function setLogLevel(level: "trace" | "debug" | "info" | "warn" | "error"): Promise<void> {
  await saveSettings({ system: { log_level: level } });
}

// ── 窗口状态 ────────────────────────────────────────────────────────────────

export async function getRestoreWindowState(): Promise<boolean> {
  const settings = await loadSettings();
  return settings.ui.restoreWindowState;
}

export async function setRestoreWindowState(enabled: boolean): Promise<void> {
  const settings = await loadSettings();
  settings.ui.restoreWindowState = enabled;
  await saveSettings({ ui: settings.ui }, settings);
}

export async function setWindowBounds(bounds: WindowBounds | null): Promise<void> {
  await saveSettings({ window: { bounds } });
}

// ── 关闭行为设置 ─────────────────────────────────────────────────────────────

export async function getCloseBehavior(): Promise<"exit" | "minimizeToTray"> {
  const settings = await loadSettings();
  return settings.ui.closeBehavior;
}

export async function setCloseBehavior(behavior: "exit" | "minimizeToTray"): Promise<void> {
  const settings = await loadSettings();
  settings.ui.closeBehavior = behavior;
  await saveSettings({ ui: settings.ui }, settings);
}