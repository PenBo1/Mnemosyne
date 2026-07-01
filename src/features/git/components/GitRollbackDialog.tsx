import { useEffect, useState } from "react";
import { cn } from "@/shared/utils";
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
import { AlertTriangleIcon, RotateCcwIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import type { RollbackMode } from "@/shared/types";

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
            <label className="text-xs font-medium text-foreground">
              {t.git.rollback.mode}
            </label>
            <div className="flex flex-col gap-1.5">
              <label
                className={cn(
                  "flex items-start gap-2 rounded-[var(--radius-3)] border p-2.5 cursor-pointer transition-colors",
                  mode === "Soft"
                    ? "border-primary bg-primary/5"
                    : "border-[var(--border-neutral-l1)] hover:bg-muted/50"
                )}
              >
                <input
                  type="radio"
                  name="rollback-mode"
                  value="Soft"
                  checked={mode === "Soft"}
                  onChange={() => setMode("Soft")}
                  className="mt-0.5"
                />
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs font-medium">{t.git.rollback.softMode}</span>
                </div>
              </label>
              <label
                className={cn(
                  "flex items-start gap-2 rounded-[var(--radius-3)] border p-2.5 cursor-pointer transition-colors",
                  mode === "Hard"
                    ? "border-destructive bg-destructive/5"
                    : "border-[var(--border-neutral-l1)] hover:bg-muted/50"
                )}
              >
                <input
                  type="radio"
                  name="rollback-mode"
                  value="Hard"
                  checked={mode === "Hard"}
                  onChange={() => setMode("Hard")}
                  className="mt-0.5"
                />
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs font-medium text-destructive">
                    {t.git.rollback.hardMode}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t.git.rollback.hardWarning}
                  </span>
                </div>
              </label>
            </div>
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
            <RotateCcwIcon />
            {t.git.rollback.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
