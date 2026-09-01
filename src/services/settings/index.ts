/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 应用设置服务 - 统一导出
 * ═══════════════════════════════════════════════════════════════════════════
 */

// ── 类型定义 ────────────────────────────────────────────────────────────────

export type {
  AiModelConfig,
  LogLevel,
  Snippet,
  WindowBounds,
  NetworkSettings,
  AppSettings,
  DeepPartial,
} from "./types";

// ── 核心设置读写 ────────────────────────────────────────────────────────────────

export { settingsStore, loadSettings, saveSettings } from "./settings-core";

// ── AI 模型管理 ────────────────────────────────────────────────────────────────

export {
  getActiveModel,
  addModel,
  removeModel,
  setActiveModel,
  updateModel,
  getCustomInstructions,
  setCustomInstructions,
} from "./ai-models";

// ── 代码片段管理 ────────────────────────────────────────────────────────────────

export {
  normalizeHandle,
  isValidHandle,
  newSnippetId,
  loadSnippets,
  saveSnippets,
} from "./snippets";

// ── 快捷键覆盖 ────────────────────────────────────────────────────────────────

export {
  getShortcutsOverrides,
  setShortcutsOverrides,
  resetShortcutOverride,
  resetAllShortcuts,
} from "./shortcuts";

// ── 网络设置 ────────────────────────────────────────────────────────────────

export {
  getNetworkSettings,
  saveNetworkSettings,
} from "./network-settings";

// ── 窗口状态 ────────────────────────────────────────────────────────────────

export {
  getLogLevel,
  setLogLevel,
  getRestoreWindowState,
  setRestoreWindowState,
  setWindowBounds,
  getCloseBehavior,
  setCloseBehavior,
} from "./window-state";