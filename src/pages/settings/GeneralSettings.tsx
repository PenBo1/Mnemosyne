import { useTheme } from "@/components/providers/Theme";
import { useI18n } from "@/lib/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { AlertTriangleIcon } from "lucide-react";
import { useGeneralSettings } from "@/hooks/useGeneralSettings";
import type { LogLevel } from "@/lib/settings";

function getLogLevelLabel(t: ReturnType<typeof useI18n>["t"], level: LogLevel): string {
  return t.common.logLevels[level] || level;
}

export function GeneralSettings() {
  const { theme, setTheme } = useTheme();
  const { locale, setLocale, t } = useI18n();
  const {
    notifications,
    logLevel,
    logLevelChanged,
    toggleNotifications,
    changeLogLevel,
  } = useGeneralSettings();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.settings.general}</h1>
        <p className="text-sm text-muted-foreground">{t.settings.generalDesc}</p>
      </div>

      {/* Language */}
      <div className="rounded-lg border bg-card">
        <div className="flex items-center justify-between px-4 py-3">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.settings.field.language}</span>
            <span className="text-xs text-muted-foreground">
              {t.settings.description.language}
            </span>
          </div>
          <Select value={locale} onValueChange={(v) => setLocale(v as "en" | "zh")}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Appearance & Notifications */}
      <div className="rounded-lg border bg-card">
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.settings.field.theme}</span>
            <span className="text-xs text-muted-foreground">
              {t.settings.description.theme}
            </span>
          </div>
          <Select value={theme} onValueChange={(v) => setTheme(v as "light" | "dark" | "system")}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="light">{t.settings.themeLight}</SelectItem>
              <SelectItem value="dark">{t.settings.themeDark}</SelectItem>
              <SelectItem value="system">{t.settings.themeSystem}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between px-4 py-3">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.settings.field.notifications}</span>
            <span className="text-xs text-muted-foreground">
              {t.settings.description.notifications}
            </span>
          </div>
          <Switch
            size="default"
            checked={notifications}
            onCheckedChange={toggleNotifications}
          />
        </div>
      </div>

      {/* Advanced */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.logLevel}</span>
              <span className="text-xs text-muted-foreground">
                {t.settings.description.logLevel}
              </span>
            </div>
            <Select value={logLevel} onValueChange={changeLogLevel}>
              <SelectTrigger className="w-32">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(["trace", "debug", "info", "warn", "error"] as LogLevel[]).map((level) => (
                  <SelectItem key={level} value={level}>{getLogLevelLabel(t, level)}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {logLevelChanged && (
            <div className="flex items-center gap-2 text-sm text-amber-600 bg-amber-50 dark:bg-amber-950 dark:text-amber-400 px-3 py-2 rounded-md mt-3">
              <AlertTriangleIcon className="size-4 shrink-0" />
              <span>{t.settings.logLevelRestartRequired}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
