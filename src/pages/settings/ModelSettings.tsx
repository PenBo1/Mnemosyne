import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  CpuIcon,
  CheckCircleIcon,
  XCircleIcon,
  Loader2Icon,
  PlusIcon,
  Trash2Icon,
  PencilIcon,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useModelSettings } from "@/hooks/useModelSettings";
import { AddModelDialog } from "./AddModelDialog";
import { EditModelDialog } from "./EditModelDialog";
import type { AiModelConfig } from "@/lib/settings";

export function ModelSettings() {
  const { t } = useI18n();
  const {
    models,
    activeModelId,
    loading,
    error,
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
      <div className="flex items-center justify-center py-8">
        <Spinner className="size-6" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <CpuIcon className="size-5" />
            {t.settings.modelSettings.title}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t.settings.modelSettings.subtitle}
          </p>
        </div>
        <Button onClick={() => setAddDialogOpen(true)} size="sm">
          <PlusIcon data-icon="inline-start" />
          {t.settings.modelSettings.addProvider}
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {activeModelId && (() => {
        const active = models.find((m) => m.id === activeModelId);
        if (!active) return null;
        return (
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-base">{t.settings.modelSettings.defaultModel}</CardTitle>
              <CardDescription>{t.settings.modelSettings.defaultModelDesc}</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <p className="text-sm">
                    <span className="text-muted-foreground">{t.settings.modelSettings.provider}: </span>
                    <span className="font-medium">{active.provider}</span>
                  </p>
                  <p className="text-sm">
                    <span className="text-muted-foreground">{t.settings.modelSettings.model}: </span>
                    <span className="font-medium">{active.name}</span>
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>
        );
      })()}

      <div className="grid gap-4">
        {models.length === 0 ? (
          <Card>
            <CardContent className="py-8 text-center text-muted-foreground">
              {t.settings.modelSettings.noProviders}
            </CardContent>
          </Card>
        ) : (
          models.map((model) => (
            <Card key={model.id}>
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <CardTitle className="text-base">{model.name}</CardTitle>
                    <Badge variant="secondary" className="text-xs capitalize">{model.provider}</Badge>
                    {activeModelId === model.id && (
                      <Badge variant="default" className="text-xs">{t.common.active}</Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleTestConnection(model.id)}
                      disabled={testing === model.id}
                    >
                      {testing === model.id ? (
                        <Loader2Icon className="size-3.5 animate-spin" />
                      ) : testResult === "success" && testing === null ? (
                        <CheckCircleIcon className="size-3.5 text-green-500" />
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
                <CardDescription>
                  {model.model} · {model.base_url || t.common.defaultUrl}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between">
                  <p className="text-xs text-muted-foreground font-mono">
                    {model.api_key.slice(0, 8)}...{model.api_key.slice(-4)}
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
              </CardContent>
            </Card>
          ))
        )}
      </div>

      <AddModelDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      <EditModelDialog
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
        model={editingModel}
      />
    </div>
  );
}
