import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Loader2Icon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useModelSettings } from "@/hooks/useModelSettings";
import type { AiModelConfig } from "@/lib/settings";

interface EditModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  model: AiModelConfig | null;
}

export function EditModelDialog({ open, onOpenChange, model }: EditModelDialogProps) {
  const { t } = useI18n();
  const { updateModel } = useModelSettings();
  const [name, setName] = useState("");
  const [modelName, setModelName] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (model) {
      setName(model.name);
      setModelName(model.model);
      setApiKey(model.api_key);
      setBaseUrl(model.base_url);
    }
  }, [model]);

  async function handleSave() {
    if (!model || !name.trim()) return;
    setSaving(true);
    try {
      await updateModel(model.id, {
        name,
        model: modelName,
        api_key: apiKey,
        base_url: baseUrl,
      });
      onOpenChange(false);
    } catch {
      // Error handled by hook
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t.settings.modelSettings.editProvider}</DialogTitle>
          <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel>{t.agents.name}</FieldLabel>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My OpenAI Config"
            />
          </Field>
          <Field>
            <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
            <Input
              value={modelName}
              onChange={(e) => setModelName(e.target.value)}
              placeholder={t.settings.modelSettings.modelPlaceholder}
            />
          </Field>
          <Field>
            <FieldLabel>{t.settings.modelSettings.apiKey}</FieldLabel>
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t.settings.modelSettings.apiKeyPlaceholder}
            />
          </Field>
          <Field>
            <FieldLabel>{t.settings.modelSettings.baseUrl}</FieldLabel>
            <Input
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder={t.settings.modelSettings.baseUrlPlaceholder}
            />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t.settings.modelSettings.cancel}
          </Button>
          <Button onClick={handleSave} disabled={!name.trim() || saving}>
            {saving ? <Loader2Icon className="size-4 animate-spin" /> : null}
            {t.settings.modelSettings.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
