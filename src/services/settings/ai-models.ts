/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AI 模型管理服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { AiModelConfig } from "./types";
import { loadSettings, saveSettings } from "./settings-core";

// ── AI 模型 CRUD 操作 ────────────────────────────────────────────────────────

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

// ── 自定义指令 ────────────────────────────────────────────────────────────────

export async function getCustomInstructions(): Promise<string> {
  const settings = await loadSettings();
  return settings.ai.custom_instructions;
}

export async function setCustomInstructions(text: string): Promise<void> {
  const settings = await loadSettings();
  settings.ai.custom_instructions = text;
  await saveSettings({ ai: settings.ai }, settings);
}