/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 应用设置类型定义
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { KeyBinding } from "@/lib/shortcuts";

// ── AI 模型配置 ────────────────────────────────────────────────────────────────

export interface AiModelConfig {
  id: string;
  name: string;
  provider: string;
  model: string;
  api_key: string;
  base_url: string;
}

// ── 日志级别 ────────────────────────────────────────────────────────────────

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

// ── 代码片段 ────────────────────────────────────────────────────────────────

export interface Snippet {
  id: string;
  handle: string;
  name: string;
  description: string;
  content: string;
}

// ── 窗口边界 ────────────────────────────────────────────────────────────────

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

// ── 网络设置 ────────────────────────────────────────────────────────────────

export interface NetworkSettings {
  proxyEnabled: boolean;
  proxyHost: string;
  proxyPort: string;
  proxyUsername: string;
  proxyPassword: string;
  timeout: string;
}

// ── 应用设置主结构 ────────────────────────────────────────────────────────

export interface AppSettings {
  ui: {
    theme: "light" | "dark" | "system";
    locale: "en" | "zh";
    notifications: boolean;
    restoreWindowState: boolean;
    closeBehavior: "exit" | "minimizeToTray";
  };
  system: {
    log_level: LogLevel;
  };
  ai: {
    models: AiModelConfig[];
    active_model_id: string | null;
    custom_instructions: string;
    snippets: Snippet[];
  };
  shortcuts: Record<string, KeyBinding[]>;
  window: {
    bounds: WindowBounds | null;
  };
  network: NetworkSettings;
}

export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};