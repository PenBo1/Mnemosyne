import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useI18n } from "@/locales/i18n";
import { toast } from "sonner";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import {
  SettingsSection,
  SettingsRow,
} from "@/features/settings/components/settings-section";
import {
  gitGetConfig,
  gitSetConfig,
  gitCheckInstalled,
  gitGetEnabled,
  gitSetEnabled,
} from "@/features/settings/services";
import type { GitConfig } from "@/features/git/types";

const DEFAULT_CONFIG: GitConfig = {
  user_name: null,
  user_email: null,
  auto_stage: false,
  commit_message_template: null,
  enable_remote: false,
};

export function GitSettings() {
  const { t } = useI18n();
  const [enabled, setEnabled] = useState(true);
  const [config, setConfig] = useState<GitConfig>(DEFAULT_CONFIG);
  const [gitInstalled, setGitInstalled] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  // 加载全局开关 + Git 安装检测 + 全局 config
  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [isEnabled, installed] = await Promise.all([
          gitGetEnabled(),
          gitCheckInstalled(),
        ]);
        if (cancelled) return;
        setEnabled(isEnabled);
        setGitInstalled(installed);
        // 读取全局 git config（home 路径，所有工作区共享）
        if (installed) {
          try {
            const cfg = await gitGetConfig("");
            if (!cancelled) setConfig(cfg);
          } catch {
            // 全局配置读取失败不阻塞页面
          }
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const handleToggleEnabled = async (checked: boolean) => {
    setEnabled(checked);
    try {
      await gitSetEnabled(checked);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      setEnabled(!checked);
      toast.error(t.common.failedToUpdate);
    }
  };

  const handleSaveConfig = async () => {
    setSaving(true);
    try {
      // 传空字符串表示写全局 git config（git config --global）
      await gitSetConfig("", config);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <div className="p-4 text-muted-foreground">{t.common.loading}</div>
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.gitLabel}</PageTitle>
          <PageDescription>{t.settings.git.description}</PageDescription>
        </PageHeading>
        <PageActions>
          {enabled && gitInstalled && (
            <Button
              size="sm"
              className="h-8 px-3 text-xs"
              disabled={saving}
              onClick={() => void handleSaveConfig()}
            >
              {saving ? t.common.saving : t.common.save}
            </Button>
          )}
        </PageActions>
      </PageHeader>

      {/* 功能开关 + 安装状态 */}
      <SettingsSection title={t.settings.git.featureToggle}>
        <SettingsRow
          label={t.settings.git.featureToggle}
          description={t.settings.git.featureToggleHint}
        >
          <Switch
            checked={enabled}
            onCheckedChange={(checked) => void handleToggleEnabled(checked)}
          />
        </SettingsRow>
        <SettingsRow
          label={t.settings.git.installStatus}
          description={
            gitInstalled
              ? t.settings.git.installed
              : t.settings.git.notInstalled
          }
        >
          {gitInstalled === false && (
            <Button
              variant="outline"
              size="sm"
              className="h-8 px-3 text-xs"
              onClick={() => {
                // 复用 git_install 命令（hook 内部已封装）
                // 这里直接调 useGit 太绕，保持简化：提示用户去 Git 页安装
                toast.info(t.settings.git.installHint);
              }}
            >
              {t.settings.git.install}
            </Button>
          )}
        </SettingsRow>
      </SettingsSection>

      {/* 全局 Git 配置（仅在启用且已安装时显示） */}
      {enabled && gitInstalled && (
        <SettingsSection title={t.settings.git.globalConfig}>
          <SettingsRow
            label={t.settings.git.userName}
            description={t.settings.git.userNameHint}
          >
            <Input
              value={config.user_name ?? ""}
              placeholder="Your Name"
              onChange={(e) =>
                setConfig({ ...config, user_name: e.target.value || null })
              }
              className="w-64"
            />
          </SettingsRow>
          <SettingsRow
            label={t.settings.git.userEmail}
            description={t.settings.git.userEmailHint}
          >
            <Input
              value={config.user_email ?? ""}
              placeholder="you@example.com"
              onChange={(e) =>
                setConfig({ ...config, user_email: e.target.value || null })
              }
              className="w-64"
            />
          </SettingsRow>
          <SettingsRow
            label={t.settings.git.commitTemplate}
            description={t.settings.git.commitTemplateHint}
          >
            <Textarea
              value={config.commit_message_template ?? ""}
              placeholder="{type}: {subject}"
              onChange={(e) =>
                setConfig({
                  ...config,
                  commit_message_template: e.target.value || null,
                })
              }
              className="w-96 min-h-[60px]"
            />
          </SettingsRow>
          <SettingsRow
            label={t.settings.git.autoStage}
            description={t.settings.git.autoStageHint}
          >
            <Switch
              checked={config.auto_stage}
              onCheckedChange={(checked) =>
                setConfig({ ...config, auto_stage: checked })
              }
            />
          </SettingsRow>
          <SettingsRow
            label={t.settings.git.enableRemote}
            description={t.settings.git.enableRemoteHint}
          >
            <Switch
              checked={config.enable_remote}
              onCheckedChange={(checked) =>
                setConfig({ ...config, enable_remote: checked })
              }
            />
          </SettingsRow>
        </SettingsSection>
      )}

      {/* 说明 */}
      {!enabled && (
        <p className="text-xs text-muted-foreground">
          {t.settings.git.disabledHint}
        </p>
      )}
    </PageContainer>
  );
}
