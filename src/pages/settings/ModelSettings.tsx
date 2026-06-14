import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Field, FieldGroup, FieldLabel, FieldSeparator } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import * as settingsStore from "@/lib/settings";
import * as providerService from "@/services/providers";
import * as agentService from "@/services/agent";
import type { AiModelConfig } from "@/lib/settings";

const PROVIDER_OPTIONS = ["openai", "ollama", "agnes"] as const;

export function ModelSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [models, setModels] = useState<AiModelConfig[]>([]);
  const [activeModelId, setActiveModelId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [newProvider, setNewProvider] = useState<string>("openai");
  const [newName, setNewName] = useState("");
  const [newModel, setNewModel] = useState("");
  const [newApiKey, setNewApiKey] = useState("");
  const [newBaseUrl, setNewBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);

  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<"success" | "failed" | null>(null);

  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<AiModelConfig | null>(null);
  const [editName, setEditName] = useState("");
  const [editModelName, setEditModelName] = useState("");
  const [editApiKey, setEditApiKey] = useState("");
  const [editBaseUrl, setEditBaseUrl] = useState("");

  useEffect(() => {
    loadModels();
  }, []);

  async function loadModels() {
    try {
      setLoading(true);
      setError(null);
      const settings = await settingsStore.loadSettings();
      setModels(settings.ai.models);
      setActiveModelId(settings.ai.active_model_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load models");
    } finally {
      setLoading(false);
    }
  }

  async function handleAddModel() {
    if (!newApiKey.trim() || !newName.trim()) return;
    setSaving(true);
    try {
      await settingsStore.addModel({
        name: newName,
        provider: newProvider,
        model: newModel || (newProvider === "openai" ? "gpt-4o" : newProvider === "ollama" ? "llama3.1" : "agnes-default"),
        api_key: newApiKey,
        base_url: newBaseUrl || "",
      });
      await providerService.refreshProviders();
      setAddDialogOpen(false);
      setNewName("");
      setNewModel("");
      setNewApiKey("");
      setNewBaseUrl("");
      await loadModels();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to add model");
    } finally {
      setSaving(false);
    }
  }

  async function handleDeleteModel(id: string) {
    try {
      await settingsStore.removeModel(id);
      await providerService.refreshProviders();
      await loadModels();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete model");
    }
  }

  async function handleSetActive(id: string) {
    try {
      await settingsStore.setActiveModel(id);
      await providerService.refreshProviders();
      await agentService.restartAgent();
      setActiveModelId(id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to set active model");
    }
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
      await providerService.testConnection({
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

  function openEditDialog(model: AiModelConfig) {
    setEditingModel(model);
    setEditName(model.name);
    setEditModelName(model.model);
    setEditApiKey(model.api_key);
    setEditBaseUrl(model.base_url);
    setEditDialogOpen(true);
  }

  async function handleEditModel() {
    if (!editingModel || !editName.trim()) return;
    setSaving(true);
    try {
      await settingsStore.updateModel(editingModel.id, {
        name: editName,
        model: editModelName,
        api_key: editApiKey,
        base_url: editBaseUrl,
      });
      await providerService.refreshProviders();
      setEditDialogOpen(false);
      setEditingModel(null);
      await loadModels();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update model");
    } finally {
      setSaving(false);
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

      {/* Active Model Summary */}
      {activeModelId && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base">{t.settings.modelSettings.defaultModel}</CardTitle>
            <CardDescription>{t.settings.modelSettings.defaultModelDesc}</CardDescription>
          </CardHeader>
          <CardContent>
            {(() => {
              const active = models.find((m) => m.id === activeModelId);
              if (!active) return <p className="text-sm text-muted-foreground">No model selected</p>;
              return (
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
              );
            })()}
          </CardContent>
        </Card>
      )}

      {/* Model List */}
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
                      <Badge variant="default" className="text-xs">Active</Badge>
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
                      onClick={() => handleDeleteModel(model.id)}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2Icon className="size-4" />
                    </Button>
                  </div>
                </div>
                <CardDescription>
                  {model.model} · {model.base_url || "default URL"}
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
                    onClick={() => handleSetActive(model.id)}
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

      {/* Add Model Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.settings.modelSettings.addProvider}</DialogTitle>
            <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.settings.modelSettings.provider}</FieldLabel>
              <Select value={newProvider} onValueChange={setNewProvider}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PROVIDER_OPTIONS.map((p) => (
                    <SelectItem key={p} value={p}>{p}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <FieldSeparator />
            <Field>
              <FieldLabel>{t.agents.name}</FieldLabel>
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="My OpenAI Config"
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
              <Input
                value={newModel}
                onChange={(e) => setNewModel(e.target.value)}
                placeholder={t.settings.modelSettings.modelPlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.apiKey}</FieldLabel>
              <Input
                type="password"
                value={newApiKey}
                onChange={(e) => setNewApiKey(e.target.value)}
                placeholder={t.settings.modelSettings.apiKeyPlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.baseUrl}</FieldLabel>
              <Input
                value={newBaseUrl}
                onChange={(e) => setNewBaseUrl(e.target.value)}
                placeholder={t.settings.modelSettings.baseUrlPlaceholder}
              />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>
              {t.settings.modelSettings.cancel}
            </Button>
            <Button onClick={handleAddModel} disabled={!newApiKey.trim() || !newName.trim() || saving}>
              {saving ? <Loader2Icon className="size-4 animate-spin" /> : null}
              {t.settings.modelSettings.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Model Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.settings.modelSettings.editProvider}</DialogTitle>
            <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.agents.name}</FieldLabel>
              <Input
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                placeholder="My OpenAI Config"
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
              <Input
                value={editModelName}
                onChange={(e) => setEditModelName(e.target.value)}
                placeholder={t.settings.modelSettings.modelPlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.apiKey}</FieldLabel>
              <Input
                type="password"
                value={editApiKey}
                onChange={(e) => setEditApiKey(e.target.value)}
                placeholder={t.settings.modelSettings.apiKeyPlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.settings.modelSettings.baseUrl}</FieldLabel>
              <Input
                value={editBaseUrl}
                onChange={(e) => setEditBaseUrl(e.target.value)}
                placeholder={t.settings.modelSettings.baseUrlPlaceholder}
              />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
              {t.settings.modelSettings.cancel}
            </Button>
            <Button onClick={handleEditModel} disabled={!editName.trim() || saving}>
              {saving ? <Loader2Icon className="size-4 animate-spin" /> : null}
              {t.settings.modelSettings.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
