import { useState } from "react";
import { useI18n } from "@/shared/i18n";
import type { LoopPattern, CreateLoopStateRequest } from "@/shared/types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
        pattern_id: selectedPatternId,
        readiness_level: readinessLevel,
        token_cap_daily: parseInt(tokenCap, 10) || 50000,
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

        <div className="flex flex-col gap-4 py-2">
          <div className="flex flex-col gap-2">
            <Label>{t.loop.patterns}</Label>
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
          </div>

          {selectedPatternId && (
            <div className="text-xs text-muted-foreground bg-muted/50 rounded p-2">
              {patterns.find((p) => p.id === selectedPatternId)?.description ??
                t.loop.noDescription}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="flex flex-col gap-2">
              <Label>{t.loop.readinessLevel}</Label>
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
            </div>

            <div className="flex flex-col gap-2">
              <Label>{t.loop.budget.cap}</Label>
              <Input
                type="number"
                value={tokenCap}
                onChange={(e) => setTokenCap(e.target.value)}
                min={1000}
                step={1000}
              />
            </div>
          </div>
        </div>

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
