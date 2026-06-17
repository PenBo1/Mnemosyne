import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import {
  isNotificationsEnabled,
  setNotificationsEnabled,
} from "@/services/notifications";
import { getLogLevel, setLogLevel } from "@/services/settings";
import type { LogLevel } from "@/lib/settings";

export function useGeneralSettings() {
  const { t } = useI18n();
  const [notifications, setNotifications] = useState(isNotificationsEnabled);
  const [logLevel, setLogLevelState] = useState<LogLevel>("info");
  const [logLevelChanged, setLogLevelChanged] = useState(false);

  useEffect(() => {
    getLogLevel().then((level) => setLogLevelState(level as LogLevel));
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
  }, [t.common.updatedSuccessfully, t.common.failedToUpdate]);

  return {
    notifications,
    logLevel,
    logLevelChanged,
    toggleNotifications,
    changeLogLevel,
  };
}
