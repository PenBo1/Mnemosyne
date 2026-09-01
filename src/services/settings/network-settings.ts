/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 网络设置管理服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { NetworkSettings } from "./types";
import { loadSettings, saveSettings } from "./settings-core";

// ── 网络设置管理 ────────────────────────────────────────────────────────

export async function getNetworkSettings(): Promise<NetworkSettings> {
  const settings = await loadSettings();
  return settings.network;
}

export async function saveNetworkSettings(network: Partial<NetworkSettings>): Promise<void> {
  const settings = await loadSettings();
  settings.network = { ...settings.network, ...network };
  await saveSettings({ network: settings.network }, settings);
}