import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  CheckCircleIcon,
  XCircleIcon,
  Loader2Icon,
  PlusIcon,
  Trash2Icon,
  PencilIcon,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useModelSettings } from "@/features/settings/hooks";
import { AddModelDialog } from "./AddModelDialog";
import { EditModelDialog } from "./EditModelDialog";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import type { AiModelConfig } from "@/shared/settings";

export function ModelSettings() {
  const { t } = useI18n();
  const {
    models,
    activeModelId,
    loading,
    removeModel,
    setActiveModel,
    testConnection,
  } = useModelSettings();

  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<AiModelConfig | null>(null);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<"success" | "failed" | null>(null);

  function openEditDialog(model: AiModelConfig) {
    setEditingModel(model);
    setEditDialogOpen(true);
  }

  async function handleTestConnection(modelId: string) {
    setTesting(modelId);
    setTestResult(null);
    try {
      const model = models.find((m) => m.id === modelId);
      if (!model) {
        setTestResult("failed");
        return;
      }
      await testConnection({
        provider: model.provider,
        apiKey: model.api_key,
        baseUrl: model.base_url,
        model: model.model,
      });
      setTestResult("success");
    } catch {
      setTestResult("failed");
    } finally {
      setTesting(null);
      setTimeout(() => setTestResult(null), 3000);
    }
  }

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.modelSettings.title}</PageTitle>
          <PageDescription>{t.settings.modelSettings.subtitle}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={() => setAddDialogOpen(true)} size="sm">
            <PlusIcon data-icon="inline-start" />
            {t.settings.modelSettings.addProvider}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 当前模型 */}
      {activeModelId && (() => {
        const active = models.find((m) => m.id === activeModelId);
        if (!active) return null;
        return (
          <Card className="py-0 gap-0">
            <CardContent className="border-b py-3">
              <span className="text-sm font-medium">{t.settings.modelSettings.defaultModel}</span>
            </CardContent>
            <CardContent className="flex items-center justify-between py-3">
              <div className="flex items-center gap-4 text-xs">
                <span className="text-muted-foreground">{t.settings.modelSettings.provider}:</span>
                <span className="font-medium">{active.provider}</span>
                <span className="text-muted-foreground">{t.settings.modelSettings.model}:</span>
                <span className="font-medium">{active.name}</span>
              </div>
            </CardContent>
          </Card>
        );
      })()}

      {/* 模型列表 */}
      {models.length === 0 ? (
        <EmptyState
          icon={<PlusIcon className="size-6" />}
          title={t.settings.modelSettings.noProviders}
          description={t.settings.modelSettings.subtitle}
        />
      ) : (
        <Card className="py-0 gap-0">
          <CardContent className="divide-y px-0">
            {models.map((model) => (
              <div key={model.id} className="flex flex-col gap-1 px-4 py-3 transition-colors hover:bg-muted/50">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">{model.name}</span>
                    <Badge variant="secondary" className="text-xs capitalize">{model.provider}</Badge>
                    {activeModelId === model.id && (
                      <Badge variant="default" className="text-xs">{t.common.active}</Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleTestConnection(model.id)}
                      disabled={testing === model.id}
                    >
                      {testing === model.id ? (
                        <Loader2Icon className="size-3.5 animate-spin" />
                      ) : testResult === "success" && testing === null ? (
                        <CheckCircleIcon className="size-3.5 text-[var(--status-success-default)]" />
                      ) : testResult === "failed" && testing === null ? (
                        <XCircleIcon className="size-3.5 text-destructive" />
                      ) : null}
                      {t.settings.modelSettings.testConnection}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => openEditDialog(model)}
                    >
                      <PencilIcon className="size-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => removeModel(model.id)}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2Icon className="size-4" />
                    </Button>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <p className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span>{model.model} · {model.base_url || t.common.defaultUrl}</span>
                    <span className="font-mono">{model.api_key.slice(0, 8)}...{model.api_key.slice(-4)}</span>
                  </p>
                  <Button
                    variant={activeModelId === model.id ? "default" : "outline"}
                    size="sm"
                    onClick={() => setActiveModel(model.id)}
                  >
                    {activeModelId === model.id
                      ? t.settings.modelSettings.defaultModel
                      : t.settings.modelSettings.selectModel}
                  </Button>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      <AddModelDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      <EditModelDialog
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
        model={editingModel}
      />
    </PageContainer>
  );
}
