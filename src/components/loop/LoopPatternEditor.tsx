import { useState } from "react";
import type { LoopPattern, CreateLoopStateRequest } from "@/types";
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

export function LoopPatternEditor({
  open,
  onOpenChange,
  patterns,
  onSubmit,
}: LoopPatternEditorProps) {
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

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>New Loop</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label>Pattern</Label>
            <Select value={selectedPatternId} onValueChange={setSelectedPatternId}>
              <SelectTrigger>
                <SelectValue placeholder="Select a pattern..." />
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
                "No description available"}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Readiness Level</Label>
              <Select
                value={readinessLevel}
                onValueChange={(v) => setReadinessLevel(v as "L0" | "L1" | "L2" | "L3")}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="L0">L0 - Draft</SelectItem>
                  <SelectItem value="L1">L1 - Report</SelectItem>
                  <SelectItem value="L2">L2 - Assisted</SelectItem>
                  <SelectItem value="L3">L3 - Unattended</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Daily Token Cap</Label>
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
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!selectedPatternId || submitting}>
            {submitting ? "Creating..." : "Create Loop"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
