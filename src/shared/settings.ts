import { LazyStore } from "@tauri-apps/plugin-store";

export interface AiModelConfig {
  id: string;
  name: string;
  provider: string;
  model: string;
  api_key: string;
  base_url: string;
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface AppSettings {
  ui: {
    theme: "light" | "dark" | "system";
    locale: "en" | "zh";
    notifications: boolean;
  };
  system: {
    log_level: LogLevel;
  };
  ai: {
    models: AiModelConfig[];
    active_model_id: string | null;
  };
}

const DEFAULTS: AppSettings = {
  ui: {
    theme: "system",
    locale: "en",
    notifications: true,
  },
  system: {
    log_level: "info",
  },
  ai: {
    models: [],
    active_model_id: null,
  },
};

const store = new LazyStore("config.json", {
  defaults: DEFAULTS as unknown as Record<string, unknown>,
  autoSave: 100,
});

export async function loadSettings(): Promise<AppSettings> {
  const entries = await store.entries<string | null>();
  const settings = { ...DEFAULTS };
  for (const [key, value] of entries) {
    if (key in settings && value !== null) {
      (settings as Record<string, unknown>)[key] = value;
    }
  }
  return settings;
}

export async function saveSettings(settings: Partial<AppSettings>): Promise<void> {
  for (const [key, value] of Object.entries(settings)) {
    await store.set(key, value);
  }
  await store.save();
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
  await saveSettings({ ai: settings.ai });
  return newModel;
}

export async function removeModel(id: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.models = settings.ai.models.filter((m) => m.id !== id);
  if (settings.ai.active_model_id === id) {
    settings.ai.active_model_id = settings.ai.models[0]?.id || null;
  }
  await saveSettings({ ai: settings.ai });
}

export async function setActiveModel(id: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.active_model_id = id;
  await saveSettings({ ai: settings.ai });
}

export async function updateModel(id: string, updates: Partial<Omit<AiModelConfig, "id">>): Promise<void> {
  const settings = await loadSettings();
  const model = settings.ai.models.find((m) => m.id === id);
  if (model) {
    Object.assign(model, updates);
    await saveSettings({ ai: settings.ai });
  }
}

export async function getLogLevel(): Promise<LogLevel> {
  const settings = await loadSettings();
  return settings.system?.log_level ?? "info";
}

export async function setLogLevel(level: LogLevel): Promise<void> {
  await saveSettings({ system: { log_level: level } });
}
