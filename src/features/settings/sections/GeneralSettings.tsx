/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GeneralSettings - 通用设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useTheme } from "@/lib/theme";
import { useI18n } from "@/locales/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { useGeneralSettings } from "@/features/settings/hooks";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import type { LogLevel } from "@/services/settings";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 获取日志级别标签
 */
function getLogLevelLabel(t: ReturnType<typeof useI18n>["t"], level: LogLevel): string {
  return t.common.logLevels[level] || level;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 通用设置页面，包含语言、主题、通知、启动和日志配置
 */
export function GeneralSettings() {
  const { theme, setTheme } = useTheme();
  const { locale, setLocale, t } = useI18n();
  const {
    notifications,
    logLevel,
    logLevelChanged,
    restoreWindow,
    toggleNotifications,
    changeLogLevel,
    toggleRestoreWindow,
  } = useGeneralSettings();

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.general}</PageTitle>
          <PageDescription>{t.settings.generalDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* ── 语言设置 ────────────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.field.language}>
        <SettingsRow label={t.settings.field.language} description={t.settings.description.language}>
          <Select value={locale} onValueChange={(v) => setLocale(v as "en" | "zh")}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
      </SettingsSection>

      {/* ── 主题和通知设置 ────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.description.theme}>
        <SettingsRow label={t.settings.field.theme} description={t.settings.description.theme}>
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
        </SettingsRow>
        <SettingsRow label={t.settings.field.notifications} description={t.settings.description.notifications}>
          <Switch size="default" checked={notifications} onCheckedChange={toggleNotifications} />
        </SettingsRow>
      </SettingsSection>

      {/* ── 启动设置 ────────────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.startup}>
        <SettingsRow label={t.settings.field.restoreWindow} description={t.settings.description.restoreWindow}>
          <Switch size="default" checked={restoreWindow} onCheckedChange={toggleRestoreWindow} />
        </SettingsRow>
      </SettingsSection>

      {/* ── 日志级别设置 ──────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.field.logLevel}>
        <div className="flex flex-col gap-3 px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.settings.field.logLevel}</span>
              <span className="text-xs text-muted-foreground">{t.settings.description.logLevel}</span>
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
            <Alert>
              <AlertDescription>{t.settings.logLevelRestartRequired}</AlertDescription>
            </Alert>
          )}
        </div>
      </SettingsSection>
    </PageContainer>
  );
}