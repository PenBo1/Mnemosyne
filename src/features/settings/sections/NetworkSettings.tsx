/**
 * ═══════════════════════════════════════════════════════════════════════════
 * NetworkSettings - 网络代理设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState } from "react";
import { useI18n } from "@/locales/i18n";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { useNetworkSettings } from "@/features/settings/hooks";

// placeholder 中展示的默认值（与 settings.ts 的 DEFAULTS 保持一致）。
const DEFAULT_PROXY_HOST = "127.0.0.1";
const DEFAULT_PROXY_PORT = "7890";
const DEFAULT_TIMEOUT_SECONDS = "30";

export function NetworkSettings() {
  const { t } = useI18n();
  const {
    proxyEnabled,
    proxyHost,
    proxyPort,
    proxyUsername,
    proxyPassword,
    timeout,
    toggleProxy,
    setProxyHost,
    setProxyPort,
    setProxyUsername,
    setProxyPassword,
    setTimeout,
    saveProxySettings,
  } = useNetworkSettings();

  const [showPassword, setShowPassword] = useState(false);

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.network}</PageTitle>
          <PageDescription>{t.settings.networkDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <SettingsSection title={t.settings.proxy}>
        <SettingsRow
          label={t.settings.field.enableProxy}
          description={t.settings.description.enableProxy}
        >
          <Switch
            size="default"
            checked={proxyEnabled}
            onCheckedChange={toggleProxy}
          />
        </SettingsRow>

        {proxyEnabled && (
          <>
            <SettingsRow
              label={t.settings.field.proxyHost}
              description={t.settings.description.proxyHost}
            >
              <Input
                type="text"
                placeholder={DEFAULT_PROXY_HOST}
                value={proxyHost}
                onChange={(e) => setProxyHost(e.target.value)}
                onBlur={saveProxySettings}
                className="w-48"
              />
            </SettingsRow>

            <SettingsRow
              label={t.settings.field.proxyPort}
              description={t.settings.description.proxyPort}
            >
              <Input
                type="number"
                placeholder={DEFAULT_PROXY_PORT}
                value={proxyPort}
                onChange={(e) => setProxyPort(e.target.value)}
                onBlur={saveProxySettings}
                className="w-24"
              />
            </SettingsRow>

            <SettingsRow
              label={t.settings.field.proxyUsername}
              description={t.settings.description.proxyUsername}
            >
              <Input
                type="text"
                placeholder={t.settings.field.proxyUsername}
                value={proxyUsername}
                onChange={(e) => setProxyUsername(e.target.value)}
                onBlur={saveProxySettings}
                className="w-48"
              />
            </SettingsRow>

            <SettingsRow
              label={t.settings.field.proxyPassword}
              description={t.settings.description.proxyPassword}
            >
              <div className="flex gap-2">
                <Input
                  type={showPassword ? "text" : "password"}
                  placeholder="••••••••"
                  value={proxyPassword}
                  onChange={(e) => setProxyPassword(e.target.value)}
                  onBlur={saveProxySettings}
                  className="w-48"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? t.common.hide : t.common.show}
                </Button>
              </div>
            </SettingsRow>
          </>
        )}
      </SettingsSection>

      <SettingsSection title={t.settings.timeout}>
        <SettingsRow
          label={t.settings.field.requestTimeout}
          description={t.settings.description.requestTimeout}
        >
          <Input
            type="number"
            placeholder={DEFAULT_TIMEOUT_SECONDS}
            value={timeout}
            onChange={(e) => setTimeout(e.target.value)}
            onBlur={saveProxySettings}
            className="w-24"
          />
          <span className="text-sm text-muted-foreground ml-2">
            {t.settings.seconds}
          </span>
        </SettingsRow>
      </SettingsSection>
    </PageContainer>
  );
}
