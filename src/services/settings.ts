import { LazyStore } from "@tauri-apps/plugin-store";
import type { KeyBinding, ShortcutId } from "@/lib/shortcuts";

export interface AiModelConfig {
  id: string;
  name: string;
  provider: string;
  model: string;
  api_key: string;
  base_url: string;
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface Snippet {
  id: string;
  handle: string;
  name: string;
  description: string;
  content: string;
}

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface NetworkSettings {
  proxyEnabled: boolean;
  proxyHost: string;
  proxyPort: string;
  proxyUsername: string;
  proxyPassword: string;
  timeout: string;
}

export interface AppSettings {
  ui: {
    theme: "light" | "dark" | "system";
    locale: "en" | "zh";
    notifications: boolean;
    restoreWindowState: boolean;
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

type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

const DEFAULTS: AppSettings = {
  ui: {
    theme: "system",
    locale: "en",
    notifications: true,
    restoreWindowState: false,
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
    proxyPort: "7890",
    proxyUsername: "",
    proxyPassword: "",
    timeout: "30",
  },
};

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

const store = new LazyStore("config.json");

export async function loadSettings(): Promise<AppSettings> {
  try {
    const entries = await store.entries<unknown>();
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
      await store.set(key, value);
    }
    await store.save();
  } catch (err) {
    console.error("Failed to save settings:", err);
    throw err;
  }
}

export async function getActiveModel(): Promise<AiModelConfig | null> {
  const settings = await loadSettings();
  if (!settings.ai.active_model_id) return null;
  return settings.ai.models.find((m) => m.id === settings.ai.active_model_id) || null;
}

export async function addModel(config: Omit<AiModelConfig, "id">): Promise<AiModelConfig> {
  const settings = await loadSettings();
  const newModel: AiModelConfig = {
    ...config,
    id: crypto.randomUUID(),
  };
  settings.ai.models.push(newModel);
  if (!settings.ai.active_model_id) {
    settings.ai.active_model_id = newModel.id;
  }
  await saveSettings({ ai: settings.ai }, settings);
  return newModel;
}

export async function removeModel(id: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.models = settings.ai.models.filter((m) => m.id !== id);
  if (settings.ai.active_model_id === id) {
    settings.ai.active_model_id = settings.ai.models[0]?.id || null;
  }
  await saveSettings({ ai: settings.ai }, settings);
}

export async function setActiveModel(id: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.active_model_id = id;
  await saveSettings({ ai: settings.ai }, settings);
}

export async function updateModel(id: string, updates: Partial<Omit<AiModelConfig, "id">>): Promise<void> {
  const settings = await loadSettings();
  const model = settings.ai.models.find((m) => m.id === id);
  if (model) {
    Object.assign(model, updates);
    await saveSettings({ ai: settings.ai }, settings);
  }
}

export async function getLogLevel(): Promise<LogLevel> {
  const settings = await loadSettings();
  return settings.system?.log_level ?? "info";
}

export async function setLogLevel(level: LogLevel): Promise<void> {
  await saveSettings({ system: { log_level: level } });
}

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

export async function getCustomInstructions(): Promise<string> {
  const settings = await loadSettings();
  return settings.ai.custom_instructions;
}

export async function setCustomInstructions(text: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.custom_instructions = text;
  await saveSettings({ ai: settings.ai }, settings);
}

const HANDLE_RE = /^[a-z0-9][a-z0-9-]*$/;

export function normalizeHandle(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function isValidHandle(h: string): boolean {
  return HANDLE_RE.test(h);
}

export function newSnippetId(): string {
  return `sn-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export async function loadSnippets(): Promise<Snippet[]> {
  const settings = await loadSettings();
  return settings.ai.snippets;
}

export async function saveSnippets(list: Snippet[]): Promise<void> {
  const settings = await loadSettings();
  settings.ai.snippets = list;
  await saveSettings({ ai: settings.ai }, settings);
}

export async function getShortcutsOverrides(): Promise<Record<string, KeyBinding[]>> {
  const settings = await loadSettings();
  return settings.shortcuts;
}

export async function setShortcutsOverrides(overrides: Record<string, KeyBinding[]>): Promise<void> {
  await saveSettings({ shortcuts: overrides });
}

export async function resetShortcutOverride(id: ShortcutId): Promise<Record<string, KeyBinding[]>> {
  const settings = await loadSettings();
  const next = { ...settings.shortcuts };
  delete next[id];
  await saveSettings({ shortcuts: next }, settings);
  return next;
}

export async function resetAllShortcuts(): Promise<void> {
  await saveSettings({ shortcuts: {} });
}

export async function getNetworkSettings(): Promise<NetworkSettings> {
  const settings = await loadSettings();
  return settings.network;
}

export async function saveNetworkSettings(network: Partial<NetworkSettings>): Promise<void> {
  const settings = await loadSettings();
  settings.network = { ...settings.network, ...network };
  await saveSettings({ network: settings.network }, settings);
}
