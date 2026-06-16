import { useState, useEffect } from "react";
import { useTheme } from "@/components/providers/Theme";
import { useI18n } from "@/lib/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import { GlobeIcon, AlertTriangleIcon } from "lucide-react";
import {
  isNotificationsEnabled,
  setNotificationsEnabled,
} from "@/services/notifications";
import { getLogLevel, setLogLevel } from "@/services/settings";
import type { LogLevel } from "@/lib/settings";

function getLogLevelLabel(t: ReturnType<typeof useI18n>["t"], level: LogLevel): string {
  return t.common.logLevels[level] || level;
}

export function GeneralSettings() {
  const { theme, setTheme } = useTheme();
  const { locale, setLocale, t } = useI18n();
  const [notifications, setNotifications] = useState(isNotificationsEnabled);
  const [logLevel, setLogLevelState] = useState<LogLevel>("info");
  const [logLevelChanged, setLogLevelChanged] = useState(false);

  useEffect(() => {
    getLogLevel().then((level) => setLogLevelState(level as LogLevel));
  }, []);

  function handleNotificationToggle(checked: boolean) {
    setNotifications(checked);
    setNotificationsEnabled(checked);
  }

  async function handleLogLevelChange(level: string) {
    const newLevel = level as LogLevel;
    setLogLevelState(newLevel);
    await setLogLevel(newLevel);
    setLogLevelChanged(true);
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <GlobeIcon className="size-5" />
          {t.settings.general}
        </h2>
        <p className="text-sm text-muted-foreground">{t.settings.generalDesc}</p>
      </div>
      <FieldGroup>
        <Field orientation="horizontal">
          <FieldLabel htmlFor="theme" className="flex-1">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.theme}</span>
              <span className="text-xs text-muted-foreground">
                {t.settings.description.theme}
              </span>
            </div>
          </FieldLabel>
          <Select value={theme} onValueChange={(v) => setTheme(v as "light" | "dark" | "system")}>
            <SelectTrigger className="w-40" id="theme">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="light">{t.settings.themeLight}</SelectItem>
              <SelectItem value="dark">{t.settings.themeDark}</SelectItem>
              <SelectItem value="system">{t.settings.themeSystem}</SelectItem>
            </SelectContent>
          </Select>
        </Field>
        <Field orientation="horizontal">
          <FieldLabel htmlFor="language" className="flex-1">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.language}</span>
              <span className="text-xs text-muted-foreground">
                {t.settings.description.language}
              </span>
            </div>
          </FieldLabel>
          <Select value={locale} onValueChange={(v) => setLocale(v as "en" | "zh")}>
            <SelectTrigger className="w-40" id="language">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
            </SelectContent>
          </Select>
        </Field>
        <Field orientation="horizontal">
          <FieldLabel htmlFor="notifications" className="flex-1">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.notifications}</span>
              <span className="text-xs text-muted-foreground">
                {t.settings.description.notifications}
              </span>
            </div>
          </FieldLabel>
          <Switch
            id="notifications"
            size="sm"
            checked={notifications}
            onCheckedChange={handleNotificationToggle}
          />
        </Field>
        <Field orientation="horizontal">
          <FieldLabel htmlFor="log-level" className="flex-1">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.logLevel}</span>
              <span className="text-xs text-muted-foreground">
                {t.settings.description.logLevel}
              </span>
            </div>
          </FieldLabel>
          <Select value={logLevel} onValueChange={handleLogLevelChange}>
            <SelectTrigger className="w-40" id="log-level">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {(["trace", "debug", "info", "warn", "error"] as LogLevel[]).map((level) => (
                <SelectItem key={level} value={level}>{getLogLevelLabel(t, level)}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        {logLevelChanged && (
          <div className="flex items-center gap-2 text-sm text-amber-600 bg-amber-50 dark:bg-amber-950 dark:text-amber-400 px-3 py-2 rounded-md">
            <AlertTriangleIcon className="size-4 shrink-0" />
            <span>{t.settings.logLevelRestartRequired}</span>
          </div>
        )}
      </FieldGroup>
    </div>
  );
}
