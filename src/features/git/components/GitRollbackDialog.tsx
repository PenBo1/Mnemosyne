import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { AlertTriangleIcon, RotateCcwIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { RollbackMode } from "@/features/git/types";

interface GitRollbackDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  commitHash: string | null;
  loading: boolean;
  onConfirm: (mode: RollbackMode) => void;
}

export function GitRollbackDialog({
  open,
  onOpenChange,
  commitHash,
  loading,
  onConfirm,
}: GitRollbackDialogProps) {
  const { t } = useI18n();
  const [mode, setMode] = useState<RollbackMode>("Soft");

  useEffect(() => {
    if (open) {
      setMode("Soft");
    }
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-[var(--status-warning-default)]">
            <AlertTriangleIcon className="size-4" />
            {t.git.rollback.title}
          </DialogTitle>
          {commitHash && (
            <DialogDescription className="font-mono">
              {commitHash.slice(0, 12)}
            </DialogDescription>
          )}
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <Alert>
            <AlertDescription>{t.git.rollback.warning}</AlertDescription>
          </Alert>

          <div className="flex flex-col gap-1.5">
            <Label className="text-xs font-medium">{t.git.rollback.mode}</Label>
            <RadioGroup
              value={mode}
              onValueChange={(v) => setMode(v as RollbackMode)}
              className="flex flex-col gap-1.5"
            >
              <Label
                htmlFor="rollback-soft"
                className={cn(
                  "flex items-start gap-2 rounded-[var(--radius-3)] border p-2.5 cursor-pointer transition-colors",
                  mode === "Soft"
                    ? "border-[var(--border-brand-l1)] bg-[var(--bg-overlay-l3)]"
                    : "border-[var(--border-neutral-l1)] hover:bg-accent"
                )}
              >
                <RadioGroupItem value="Soft" id="rollback-soft" className="mt-0.5" />
                <span className="text-xs font-medium">{t.git.rollback.softMode}</span>
              </Label>
              <Label
                htmlFor="rollback-hard"
                className={cn(
                  "flex items-start gap-2 rounded-[var(--radius-3)] border p-2.5 cursor-pointer transition-colors",
                  mode === "Hard"
                    ? "border-destructive bg-destructive/5"
                    : "border-[var(--border-neutral-l1)] hover:bg-accent"
                )}
              >
                <RadioGroupItem value="Hard" id="rollback-hard" className="mt-0.5" />
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs font-medium text-destructive">
                    {t.git.rollback.hardMode}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t.git.rollback.hardWarning}
                  </span>
                </div>
              </Label>
            </RadioGroup>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={loading}>
            {t.git.rollback.cancel}
          </Button>
          <Button
            variant={mode === "Hard" ? "destructive" : "default"}
            onClick={() => onConfirm(mode)}
            disabled={loading || !commitHash}
          >
            <RotateCcwIcon className="size-4" />
            {t.git.rollback.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
