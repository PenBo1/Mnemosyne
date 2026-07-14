import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  CheckCircleIcon,
  XCircleIcon,
  Loader2Icon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { AiModelConfig } from "@/services/settings";
import type { ReactNode } from "react";

interface ModelCardProps {
  model: AiModelConfig;
  isActive: boolean;
  testing: boolean;
  testResult: "success" | "failed" | null;
  onTest: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onSetActive: () => void;
}

function ModelCard({
  model,
  isActive,
  testing,
  testResult,
  onTest,
  onEdit,
  onDelete,
  onSetActive,
}: ModelCardProps) {
  const { t } = useI18n();

  function renderTestIcon(): ReactNode {
    if (testing) {
      return <Loader2Icon className="size-3.5 animate-spin" />;
    }
    if (testResult === "success") {
      return <CheckCircleIcon className="size-3.5 text-[var(--status-success-default)]" />;
    }
    if (testResult === "failed") {
      return <XCircleIcon className="size-3.5 text-destructive" />;
    }
    return null;
  }

  return (
    <Card className="py-0">
      <CardContent className="divide-y px-0">
        <div className="flex flex-col gap-1 px-4 py-3 transition-colors hover:bg-accent">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">{model.name}</span>
              <Badge variant="secondary" className="text-xs capitalize">
                {model.provider}
              </Badge>
              {isActive && (
                <Badge variant="default" className="text-xs">
                  {t.common.active}
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={onTest}
                disabled={testing}
              >
                {renderTestIcon()}
                {t.settings.modelSettings.testConnection}
              </Button>
              <Button variant="ghost" size="icon-sm" onClick={onEdit}>
                <PencilIcon className="size-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onDelete}
                className="text-destructive hover:text-destructive"
              >
                <Trash2Icon className="size-4" />
              </Button>
            </div>
          </div>
          <div className="flex items-center justify-between">
            <p className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>
                {model.model} · {model.base_url || t.common.defaultUrl}
              </span>
              <span className="font-mono">
                {model.api_key.slice(0, 8)}...{model.api_key.slice(-4)}
              </span>
            </p>
            <Button
              variant={isActive ? "default" : "outline"}
              size="sm"
              onClick={onSetActive}
            >
              {isActive
                ? t.settings.modelSettings.defaultModel
                : t.settings.modelSettings.selectModel}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export { ModelCard };