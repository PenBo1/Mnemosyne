/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 快捷键覆盖管理服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { KeyBinding } from "@/lib/shortcuts";
import type { ShortcutId } from "@/lib/shortcuts";
import { loadSettings, saveSettings } from "./settings-core";

// ── 快捷键覆盖管理 ────────────────────────────────────────────────────────

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