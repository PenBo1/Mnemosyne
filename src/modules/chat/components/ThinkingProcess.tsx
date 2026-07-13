import { useState } from "react";
import {
  ChevronDown,
  FileText,
  Search,
  Loader2,
  Check,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";

/**
 * ThinkingProcess: collapsible panel showing AI reasoning/thinking steps.
 *
 * Each step represents a line from the reasoning stream (typically separated by newlines).
 * Steps show progress indicators (spinning loader for in-progress, check for completed).
 * Matches TRAE Code's "Thinking Process" section.
 */
export function ThinkingProcess({
  reasoning,
  isStreaming,
}: {
  reasoning?: string;
  isStreaming: boolean;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(true);

  const steps = parseReasoningSteps(reasoning);
  const hasSteps = steps.length > 0;

  if (!hasSteps && !isStreaming) return null;

  return (
    <div className="mb-2 border-b border-[var(--border-neutral-l1)] pb-2">
      {/* Toggle button */}
      <Button
        variant="ghost"
        size="sm"
        className="w-full justify-start font-medium"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <ChevronDown
          className={cn(
            "transition-transform duration-200",
            open ? "" : "-rotate-90"
          )}
        />
        <span>{t.agentChat.reasoningTitle}</span>
        {isStreaming && (
          <Loader2 className="animate-spin text-[var(--status-primary-default)]" />
        )}
      </Button>

      {/* Steps */}
      {open && (
        <div className="mt-2 flex flex-col gap-2">
          {steps.map((step, idx) => {
            const isLast = idx === steps.length - 1;
            const isLastAndStreaming = isLast && isStreaming;

            return (
              <div
                key={idx}
                className="flex items-start gap-2 text-xs leading-relaxed text-[var(--text-secondary)]"
              >
                {/* Status icon */}
                <span className="mt-0.5 shrink-0">
                  {isLastAndStreaming ? (
                    <Loader2 className="size-3 animate-spin text-[var(--status-primary-default)]" />
                  ) : (
                    <Check className="size-3 text-[var(--status-success-default)]" />
                  )}
                </span>
                {/* Step content */}
                <span className={isLastAndStreaming ? "text-[var(--text-secondary)]" : "text-[var(--text-tertiary)]"}>
                  {step.text}
                </span>
                {/* Step metadata (file count, search count) */}
                {step.filesRead > 0 && (
                  <span className="mt-0.5 shrink-0 text-[10px] text-[var(--text-tertiary)]">
                    <FileText className="mr-0.5 inline-block size-3" />
                    {t.agentChat.referenceFiles.replace("{count}", String(step.filesRead))}
                  </span>
                )}
                {step.searchCount > 0 && (
                  <span className="mt-0.5 shrink-0 text-[10px] text-[var(--text-tertiary)]">
                    <Search className="mr-0.5 inline-block size-3" />
                    {t.agentChat.referenceSearches.replace("{count}", String(step.searchCount))}
                  </span>
                )}
              </div>
            );
          })}

          {/* Show animated dots when streaming and no steps yet */}
          {isStreaming && steps.length === 0 && (
            <span className="inline-flex items-center gap-1.5 text-xs text-[var(--text-tertiary)]">
              <Loader2 className="size-3 animate-spin text-[var(--status-primary-default)]" />
              <span>{t.agentChat.thinking}</span>
            </span>
          )}
        </div>
      )}
    </div>
  );
}

interface ReasoningStep {
  text: string;
  filesRead: number;
  searchCount: number;
}

function parseReasoningSteps(reasoning?: string): ReasoningStep[] {
  if (!reasoning) return [];

  return reasoning
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => ({
      text: line.trim(),
      filesRead: extractCount(line, /已读取\s*(\d+)\s*个文件/) ?? extractCount(line, /Read\s+(\d+)\s+file/) ?? 0,
      searchCount: extractCount(line, /已搜索\s*(\d+)\s*次文件/) ?? extractCount(line, /Searched\s+(\d+)\s+time/) ?? 0,
    }));
}

function extractCount(text: string, pattern: RegExp): number | null {
  const match = pattern.exec(text);
  return match ? parseInt(match[1], 10) : null;
}
