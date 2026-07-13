import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { GitBranchIcon, CheckCircle2Icon, XCircleIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useWorkspaceStore } from "@/stores/workspace";
import { useGitSettings } from "@/features/settings/hooks";
import type { GitConfig } from "@/shared/types";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";

export function GitSettings() {
  const { t } = useI18n();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const activeWorkspace = useMemo(
    () => workspaces.find((ws) => ws.id === activeWorkspaceId) ?? null,
    [workspaces, activeWorkspaceId]
  );
  const workspacePath = activeWorkspace?.path ?? "";

  const {
    config,
    loading,
    saving,
    gitInstalled,
    loadConfig,
    saveConfig,
    checkInstalled,
    defaultConfig,
  } = useGitSettings();

  const [draft, setDraft] = useState<GitConfig>(defaultConfig);

  useEffect(() => {
    if (workspacePath) {
      loadConfig(workspacePath);
    }
  }, [workspacePath, loadConfig]);

  useEffect(() => {
    if (config) {
      setDraft(config);
    } else {
      setDraft(defaultConfig);
    }
  }, [config, defaultConfig]);

  function updateField<K extends keyof GitConfig>(key: K, value: GitConfig[K]) {
    setDraft((prev) => ({ ...prev, [key]: value }));
  }

  async function handleSave() {
    if (!workspacePath) return;
    await saveConfig(workspacePath, draft);
  }

  if (!workspacePath) {
    return (
      <PageContainer scrollable={false}>
        <PageHeader>
          <PageHeading>
            <PageTitle>
              <GitBranchIcon className="size-5" />
              {t.settings.git.title}
            </PageTitle>
          </PageHeading>
        </PageHeader>
        <EmptyState
          icon={<GitBranchIcon className="size-5" />}
          title={t.settings.git.noWorkspace}
        />
      </PageContainer>
    );
  }

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <PageHeader>
          <PageHeading>
            <PageTitle>
              <GitBranchIcon className="size-5" />
              {t.settings.git.title}
            </PageTitle>
          </PageHeading>
        </PageHeader>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GitBranchIcon className="size-5" />
            {t.settings.git.title}
          </PageTitle>
          <PageDescription>{activeWorkspace?.name ?? workspacePath}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button variant="outline" size="sm" onClick={checkInstalled}>
            {t.settings.git.checkInstalled}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={saving}>
            {saving ? t.common.loading : t.settings.git.save}
          </Button>
        </PageActions>
      </PageHeader>

      {gitInstalled !== null && (
        <Card>
          <CardContent className="flex items-center gap-2">
            {gitInstalled ? (
              <>
                <CheckCircle2Icon className="size-4 shrink-0 text-emerald-500" />
                <span className="text-sm">
                  {t.settings.git.installed
                    .replace("{version}", "")
                    .replace(/[:：]\s*$/, "")}
                </span>
              </>
            ) : (
              <>
                <XCircleIcon className="size-4 shrink-0 text-destructive" />
                <span className="text-sm">{t.settings.git.notInstalled}</span>
              </>
            )}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="flex flex-col gap-3 py-3">
          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">{t.settings.git.userName}</span>
            <Input
              value={draft.user_name ?? ""}
              placeholder={t.settings.git.userName}
              onChange={(e) => updateField("user_name", e.target.value || null)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">{t.settings.git.userEmail}</span>
            <Input
              type="email"
              value={draft.user_email ?? ""}
              placeholder={t.settings.git.userEmail}
              onChange={(e) => updateField("user_email", e.target.value || null)}
            />
          </div>
        </CardContent>
      </Card>

      <Card className="py-0 gap-0">
        <CardContent className="flex items-center justify-between border-b py-3">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.settings.git.autoStage}</span>
            <span className="text-xs text-muted-foreground">
              {t.settings.git.autoStageDesc}
            </span>
          </div>
          <Switch
            checked={draft.auto_stage}
            onCheckedChange={(checked) => updateField("auto_stage", checked)}
          />
        </CardContent>
        <CardContent className="flex items-center justify-between py-3">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.settings.git.enableRemote}</span>
            <span className="text-xs text-muted-foreground">
              {t.settings.git.enableRemoteDesc}
            </span>
          </div>
          <Switch
            checked={draft.enable_remote}
            onCheckedChange={(checked) => updateField("enable_remote", checked)}
          />
        </CardContent>
        {draft.enable_remote && (
          <CardContent className="py-3">
            <Alert>
              <AlertDescription>{t.settings.git.enableRemoteWarning}</AlertDescription>
            </Alert>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardContent className="flex flex-col gap-1.5 py-3">
          <span className="text-sm font-medium">{t.settings.git.commitTemplate}</span>
          <span className="text-xs text-muted-foreground">
            {t.settings.git.commitTemplateDesc}
          </span>
          <Textarea
            value={draft.commit_message_template ?? ""}
            placeholder={t.settings.git.commitTemplate}
            rows={4}
            onChange={(e) =>
              updateField("commit_message_template", e.target.value || null)
            }
          />
        </CardContent>
      </Card>
    </PageContainer>
  );
}
