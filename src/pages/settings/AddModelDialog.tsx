import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
import { Loader2Icon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useModelSettings } from "@/features/settings/hooks";

const PROVIDER_OPTIONS = ["openai", "ollama", "agnes"] as const;

interface AddModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AddModelDialog({ open, onOpenChange }: AddModelDialogProps) {
  const { t } = useI18n();
  const { addModel } = useModelSettings();
  const [provider, setProvider] = useState<string>("openai");
  const [name, setName] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);

  async function handleSave() {
    if (!apiKey.trim() || !name.trim()) return;
    setSaving(true);
    try {
      await addModel({
        name,
        provider,
        model: model || (provider === "openai" ? "gpt-4o" : provider === "ollama" ? "llama3.1" : "agnes-default"),
        api_key: apiKey,
        base_url: baseUrl || "",
      });
      onOpenChange(false);
      setName("");
      setModel("");
      setApiKey("");
      setBaseUrl("");
    } catch {
      // 错误由 hook 处理
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t.settings.modelSettings.addProvider}</DialogTitle>
          <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel>{t.settings.modelSettings.provider}</FieldLabel>
            <Select value={provider} onValueChange={setProvider}>
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
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My OpenAI Config"
            />
          </Field>
          <Field>
            <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
            <Input
              value={model}
              onChange={(e) => setModel(e.target.value)}
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
          <Button onClick={handleSave} disabled={!apiKey.trim() || !name.trim() || saving}>
            {saving ? <Loader2Icon className="size-4 animate-spin" /> : null}
            {t.settings.modelSettings.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
