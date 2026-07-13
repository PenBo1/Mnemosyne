import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  isNotificationsEnabled,
  setNotificationsEnabled,
} from "@/features/tools/services/notifications";
import { getLogLevel, setLogLevel } from "@/features/settings/services";
import {
  getRestoreWindowState,
  setRestoreWindowState,
} from "@/services/settings";
import type { LogLevel } from "@/services/settings";

export function useGeneralSettings() {
  const { t } = useI18n();
  const [notifications, setNotifications] = useState(isNotificationsEnabled);
  const [logLevel, setLogLevelState] = useState<LogLevel>("info");
  const [logLevelChanged, setLogLevelChanged] = useState(false);
  const [restoreWindow, setRestoreWindowValue] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getLogLevel().then((level) => {
      if (!cancelled) setLogLevelState(level as LogLevel);
    });
    void getRestoreWindowState().then((enabled) => {
      if (!cancelled) setRestoreWindowValue(enabled);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const toggleNotifications = useCallback((checked: boolean) => {
    setNotifications(checked);
    setNotificationsEnabled(checked);
  }, []);

  const changeLogLevel = useCallback(async (level: string) => {
    try {
      const newLevel = level as LogLevel;
      setLogLevelState(newLevel);
      await setLogLevel(newLevel);
      setLogLevelChanged(true);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, []);

  const toggleRestoreWindow = useCallback(async (enabled: boolean) => {
    setRestoreWindowValue(enabled);
    try {
      await setRestoreWindowState(enabled);
    } catch (e) {
      console.error("[general] set restore window state failed", e);
      toast.error(t.common.failedToSave);
    }
  }, []);

  return {
    notifications,
    logLevel,
    logLevelChanged,
    restoreWindow,
    toggleNotifications,
    changeLogLevel,
    toggleRestoreWindow,
  };
}
