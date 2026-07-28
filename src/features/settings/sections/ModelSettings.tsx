/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ModelSettings - AI 模型设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { PlusIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useModelSettings } from "@/features/settings/hooks";
import { ModelCard } from "@/features/settings/components/model-card";
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
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { LoadingState, EmptyState } from "@/components/shared/state";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AiModelConfig } from "@/services/settings";

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
  const testResultTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 组件卸载时清理测试结果定时器，避免卸载后 setState
  useEffect(() => {
    return () => {
      if (testResultTimerRef.current) clearTimeout(testResultTimerRef.current);
    };
  }, []);

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
      if (testResultTimerRef.current) clearTimeout(testResultTimerRef.current);
      testResultTimerRef.current = setTimeout(() => setTestResult(null), 3000);
    }
  }

  if (loading) {
    return (
      <PageContainer>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
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

      {models.length > 0 && (
        <SettingsSection title={t.settings.modelSettings.defaultModel}>
          <SettingsRow
            label={t.settings.modelSettings.defaultModel}
            description={t.settings.modelSettings.defaultModelDesc}
          >
            <Select value={activeModelId ?? ""} onValueChange={setActiveModel}>
              <SelectTrigger className="w-56">
                <SelectValue placeholder={t.settings.modelSettings.selectModel} />
              </SelectTrigger>
              <SelectContent>
                {models.map((m) => (
                  <SelectItem key={m.id} value={m.id}>
                    {m.name} · {m.model}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </SettingsRow>
        </SettingsSection>
      )}

      {models.length === 0 ? (
        <EmptyState
          icon={<PlusIcon className="size-6" />}
          title={t.settings.modelSettings.noProviders}
          description={t.settings.modelSettings.subtitle}
        />
      ) : (
        <div className="flex flex-col gap-3">
          {models.map((model) => (
            <ModelCard
              key={model.id}
              model={model}
              isActive={activeModelId === model.id}
              testing={testing === model.id}
              testResult={testResult}
              onTest={() => handleTestConnection(model.id)}
              onEdit={() => openEditDialog(model)}
              onDelete={() => removeModel(model.id)}
              onSetActive={() => setActiveModel(model.id)}
            />
          ))}
        </div>
      )}

      <AddModelDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      <EditModelDialog key={editingModel?.id ?? 'none'} open={editDialogOpen} onOpenChange={setEditDialogOpen} model={editingModel} />
    </PageContainer>
  );
}