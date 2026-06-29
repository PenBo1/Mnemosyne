import { useTheme } from "@/shared/theme";
import { useI18n } from "@/shared/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent } from "@/components/ui/card";
import { AlertTriangleIcon } from "lucide-react";
import { useGeneralSettings } from "@/features/settings/hooks";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import type { LogLevel } from "@/shared/settings";

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
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.general}</PageTitle>
          <PageDescription>{t.settings.generalDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* 语言 */}
      <Card>
        <CardContent className="flex items-center justify-between">
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
        </CardContent>
      </Card>

      {/* 外观与通知 */}
      <Card className="py-0 gap-0">
        <CardContent className="flex items-center justify-between border-b py-3">
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
        </CardContent>
        <CardContent className="flex items-center justify-between py-3">
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
        </CardContent>
      </Card>

      {/* 高级 */}
      <Card className="py-0 gap-0">
        <CardContent className="flex flex-col gap-3 py-3">
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
            <div className="flex items-center gap-2 rounded-[var(--radius-4)] bg-muted px-3 py-2 text-sm text-muted-foreground">
              <AlertTriangleIcon className="size-4 shrink-0" />
              <span>{t.settings.logLevelRestartRequired}</span>
            </div>
          )}
        </CardContent>
      </Card>
    </PageContainer>
  );
}
