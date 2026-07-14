import { useState } from "react";
import { useI18n } from "@/locales/i18n";
import type { LoopPattern, CreateLoopStateRequest } from "@/features/loop/types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface LoopPatternEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  patterns: LoopPattern[];
  onSubmit: (req: CreateLoopStateRequest) => Promise<void>;
}

const READINESS_LEVELS = ["L0", "L1", "L2", "L3"] as const;

export function LoopPatternEditor({
  open,
  onOpenChange,
  patterns,
  onSubmit,
}: LoopPatternEditorProps) {
  const { t } = useI18n();
  const [selectedPatternId, setSelectedPatternId] = useState("");
  const [readinessLevel, setReadinessLevel] = useState<"L0" | "L1" | "L2" | "L3">("L0");
  const [tokenCap, setTokenCap] = useState("50000");
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!selectedPatternId) return;
    setSubmitting(true);
    try {
      await onSubmit({
        patternId: selectedPatternId,
        readinessLevel: readinessLevel,
        tokenCapDaily: parseInt(tokenCap, 10) || 50000,
      });
      onOpenChange(false);
      setSelectedPatternId("");
    } finally {
      setSubmitting(false);
    }
  };

  const getReadinessLabel = (level: string) =>
    t.loop.readiness[level as keyof typeof t.loop.readiness];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{t.loop.newLoop}</DialogTitle>
        </DialogHeader>

        <FieldGroup className="py-2">
          <Field>
            <FieldLabel>{t.loop.patterns}</FieldLabel>
            <Select value={selectedPatternId} onValueChange={setSelectedPatternId}>
              <SelectTrigger>
                <SelectValue placeholder={t.loop.selectPatternPlaceholder} />
              </SelectTrigger>
              <SelectContent>
                {patterns.map((p) => (
                  <SelectItem key={p.id} value={p.id}>
                    {p.name} ({p.cadence})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {selectedPatternId && (
            <div className="text-xs text-muted-foreground bg-[var(--bg-overlay-l1)] rounded p-2">
              {patterns.find((p) => p.id === selectedPatternId)?.description ??
                t.loop.noDescription}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <Field>
              <FieldLabel>{t.loop.readinessLevel}</FieldLabel>
              <Select
                value={readinessLevel}
                onValueChange={(v) => setReadinessLevel(v as "L0" | "L1" | "L2" | "L3")}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {READINESS_LEVELS.map((level) => (
                    <SelectItem key={level} value={level}>
                      {level} — {getReadinessLabel(level)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>

            <Field>
              <FieldLabel>{t.loop.budget.cap}</FieldLabel>
              <Input
                type="number"
                value={tokenCap}
                onChange={(e) => setTokenCap(e.target.value)}
                min={1000}
                step={1000}
              />
            </Field>
          </div>
        </FieldGroup>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t.common.cancel}
          </Button>
          <Button onClick={handleSubmit} disabled={!selectedPatternId || submitting}>
            {submitting ? t.common.loading : t.loop.newLoop}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
