import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ShieldCheckIcon, InfoIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";

export function SystemSettings() {
  const { t } = useI18n();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <ShieldCheckIcon className="size-5" />
          {t.settings.system}
        </h2>
        <p className="text-sm text-muted-foreground">{t.settings.systemDesc}</p>
      </div>
      <FieldGroup>
        <Field orientation="horizontal">
          <FieldLabel className="flex-1 text-muted-foreground">App</FieldLabel>
          <Badge variant="secondary">Mnemosyne v0.1.0</Badge>
        </Field>
        <Separator />
        <Field orientation="horizontal">
          <FieldLabel className="flex-1 text-muted-foreground">Framework</FieldLabel>
          <span className="text-sm">Tauri v2 + React 19</span>
        </Field>
        <Separator />
        <Field orientation="horizontal">
          <FieldLabel className="flex-1 text-muted-foreground">Runtime</FieldLabel>
          <span className="text-sm">Vite + TypeScript</span>
        </Field>
      </FieldGroup>

      <div>
        <h3 className="text-sm font-semibold flex items-center gap-2">
          <InfoIcon className="size-4" />
          {t.settings.systemDesc}
        </h3>
        <p className="text-sm text-muted-foreground mt-1">
          {t.ai.comingSoon}
        </p>
      </div>
    </div>
  );
}
