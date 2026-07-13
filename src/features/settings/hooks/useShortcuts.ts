import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  getShortcutsOverrides,
  setShortcutsOverrides,
  resetShortcutOverride,
  resetAllShortcuts,
} from "@/services/settings";
import type { KeyBinding, ShortcutId } from "@/lib/shortcuts";

/** 快捷键覆盖 CRUD：持久化到 config.json */
export function useShortcuts() {
  const { t } = useI18n();
  const [overrides, setOverrides] = useState<Record<string, KeyBinding[]>>({});
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getShortcutsOverrides().then((map) => {
      if (cancelled) return;
      setOverrides(map);
      setLoaded(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // 录制：覆盖某 id 的绑定（单绑定）
  const record = useCallback(async (id: ShortcutId, binding: KeyBinding) => {
    const next = { ...overrides, [id]: [binding] };
    try {
      await setShortcutsOverrides(next);
      setOverrides(next);
    } catch (e) {
      console.error("[shortcuts] record failed", e);
      toast.error(t.common.failedToSave);
    }
  }, [overrides]);

  // 清除：置为空数组，表示该快捷键无绑定
  const clear = useCallback(async (id: ShortcutId) => {
    const next = { ...overrides, [id]: [] };
    try {
      await setShortcutsOverrides(next);
      setOverrides(next);
    } catch (e) {
      console.error("[shortcuts] clear failed", e);
      toast.error(t.common.failedToSave);
    }
  }, [overrides]);

  // 重置单个：移除覆盖，回到默认绑定
  const reset = useCallback(async (id: ShortcutId) => {
    try {
      const next = await resetShortcutOverride(id);
      setOverrides(next);
    } catch (e) {
      console.error("[shortcuts] reset failed", e);
      toast.error(t.common.failedToSave);
    }
  }, []);

  // 重置全部
  const resetAll = useCallback(async () => {
    try {
      await resetAllShortcuts();
      setOverrides({});
      toast.success(t.shortcuts.resetAllDone);
    } catch (e) {
      console.error("[shortcuts] resetAll failed", e);
      toast.error(t.common.failedToSave);
    }
  }, []);

  return { overrides, loaded, record, clear, reset, resetAll };
}
