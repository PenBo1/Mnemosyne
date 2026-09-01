/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 应用设置核心服务 - 提供配置的持久化存储与访问
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { LazyStore } from "@tauri-apps/plugin-store";
import type { KeyBinding } from "@/lib/shortcuts";
import type { AppSettings, DeepPartial } from "./types";

// ── 默认值 ────────────────────────────────────────────────────────────────

const DEFAULT_PROXY_PORT = "7890";
const DEFAULT_TIMEOUT_SECONDS = "30";

const DEFAULTS: AppSettings = {
  ui: {
    theme: "system",
    locale: "en",
    notifications: true,
    restoreWindowState: false,
    closeBehavior: "exit",
  },
  system: {
    log_level: "info",
  },
  ai: {
    models: [],
    active_model_id: null,
    custom_instructions: "",
    snippets: [],
  },
  shortcuts: {},
  window: {
    bounds: null,
  },
  network: {
    proxyEnabled: false,
    proxyHost: "",
    proxyPort: DEFAULT_PROXY_PORT,
    proxyUsername: "",
    proxyPassword: "",
    timeout: DEFAULT_TIMEOUT_SECONDS,
  },
};

// ── 辅助函数 ────────────────────────────────────────────────────────

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function deepMerge<T>(base: T, override: unknown): T {
  if (!isPlainObject(base) || !isPlainObject(override)) {
    return (override ?? base) as T;
  }
  const out: Record<string, unknown> = { ...base };
  for (const [k, v] of Object.entries(override)) {
    if (!(k in out) || v === null || v === undefined) continue;
    const cur = out[k];
    out[k] = isPlainObject(cur) && isPlainObject(v) ? deepMerge(cur, v) : v;
  }
  return out as T;
}

// ── 存储实例（导出供其他模块使用）────────────────────────────────────────

export const settingsStore = new LazyStore("config.json");

// ── 设置读写 ────────────────────────────────────────────────────────

export async function loadSettings(): Promise<AppSettings> {
  try {
    const entries = await settingsStore.entries<unknown>();
    const stored: Record<string, unknown> = {};
    for (const [key, value] of entries) {
      if (value !== null && value !== undefined) {
        stored[key] = value;
      }
    }
    const merged = deepMerge(DEFAULTS, stored);
    if (isPlainObject(stored.shortcuts)) {
      merged.shortcuts = stored.shortcuts as Record<string, KeyBinding[]>;
    }
    return merged;
  } catch (err) {
    console.error("Failed to load settings:", err);
    return { ...DEFAULTS };
  }
}

export async function saveSettings(settings: DeepPartial<AppSettings>, current?: AppSettings): Promise<void> {
  try {
    const base = current ?? await loadSettings();
    const merged = deepMerge(base, settings);
    if (settings.shortcuts !== undefined) {
      merged.shortcuts = settings.shortcuts as Record<string, KeyBinding[]>;
    }
    for (const [key, value] of Object.entries(merged)) {
      await settingsStore.set(key, value);
    }
    await settingsStore.save();
  } catch (err) {
    console.error("Failed to save settings:", err);
    throw err;
  }
}