import { Badge } from "@/components/ui/badge";
import { InfoIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";

export function SystemSettings() {
  const { t } = useI18n();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.settings.system}</h1>
        <p className="text-sm text-muted-foreground">{t.settings.systemDesc}</p>
      </div>

      {/* System Info */}
      <div className="rounded-lg border bg-card">
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <span className="text-sm font-medium">{t.common.app}</span>
          <Badge variant="secondary">Mnemosyne v0.1.0</Badge>
        </div>

        <div className="flex items-center justify-between px-4 py-3 border-b">
          <span className="text-sm font-medium">{t.common.framework}</span>
          <span className="text-sm text-muted-foreground">Tauri v2 + React 19</span>
        </div>

        <div className="flex items-center justify-between px-4 py-3">
          <span className="text-sm font-medium">{t.common.runtime}</span>
          <span className="text-sm text-muted-foreground">Vite + TypeScript</span>
        </div>
      </div>

      {/* Coming Soon */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-1">
            <InfoIcon className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">{t.settings.systemDesc}</span>
          </div>
          <p className="text-sm text-muted-foreground">
            {t.ai.comingSoon}
          </p>
        </div>
      </div>
    </div>
  );
}
