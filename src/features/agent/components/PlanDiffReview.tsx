/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PlanDiffReview - 计划差异审查组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useMemo, memo, useCallback } from "react";
import { FileEdit, FilePlus, FolderPlus, Trash2, Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useI18n } from "@/locales/i18n";
import type { QueuedEdit } from "@/features/agent/store/plan-store";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface PlanRowProps {
  item: QueuedEdit;
  onReject: (id: string) => void;
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 计算文本行差异
 */
function diffLines(original: string, proposed: string): Array<{ text: string; kind: "same" | "added" | "removed" }> {
  const oldLines = original.split("\n");
  const newLines = proposed.split("\n");
  const oldSet = new Set(oldLines);
  const newSet = new Set(newLines);
  const result: Array<{ text: string; kind: "same" | "added" | "removed" }> = [];
  const max = Math.max(oldLines.length, newLines.length);
  for (let i = 0; i < max; i++) {
    const o = oldLines[i];
    const n = newLines[i];
    if (o === n) {
      if (o !== undefined) result.push({ text: o, kind: "same" });
      continue;
    }
    if (o !== undefined && !newSet.has(o)) {
      result.push({ text: o, kind: "removed" });
    }
    if (n !== undefined && !oldSet.has(n)) {
      result.push({ text: n, kind: "added" });
    }
  }
  return result.slice(0, 80);
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 单个计划行组件
 */
const PlanRow = memo(function PlanRow({ item, onReject }: PlanRowProps) {
  const [expanded, setExpanded] = useState(false);
  const Icon = item.kind === "create_directory" ? FolderPlus : item.isNewFile ? FilePlus : FileEdit;
  const diff = useMemo(
    () => item.kind === "create_directory" ? [] : diffLines(item.originalContent, item.proposedContent),
    [item.kind, item.originalContent, item.proposedContent]
  );
  const { added, removed } = useMemo(() => {
    let a = 0, r = 0;
    for (const d of diff) {
      if (d.kind === "added") a++;
      else if (d.kind === "removed") r++;
    }
    return { added: a, removed: r };
  }, [diff]);

  return (
    <div className="rounded border border-[var(--border-neutral-l1)] bg-[var(--bg-elevated-default)]">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="flex w-full items-center gap-2 p-2 text-left hover:bg-[var(--bg-elevated-hover)]"
      >
        <Icon className="size-4 shrink-0 text-[var(--text-tertiary)]" />
        <span className="min-w-0 flex-1 truncate font-mono text-xs">{item.path}</span>
        <Badge variant="outline" className="shrink-0">
          {item.kind}
        </Badge>
        {added > 0 && (
          <span className="shrink-0 text-xs text-[var(--color-success)]">+{added}</span>
        )}
        {removed > 0 && (
          <span className="shrink-0 text-xs text-[var(--color-error)]">-{removed}</span>
        )}
        <X
          className="size-3.5 shrink-0 cursor-pointer text-[var(--text-tertiary)] hover:text-red-600"
          onClick={(e) => {
            e.stopPropagation();
            onReject(item.id);
          }}
        />
      </button>
      {expanded && diff.length > 0 && (
        <pre className="max-h-60 overflow-auto border-t border-[var(--border-neutral-l1)] p-2 text-xs">
          {diff.map((d, i) => (
            <div
              key={i}
              className={cn(
                "whitespace-pre-wrap break-words",
                d.kind === "added" && "bg-[var(--color-success-bg)]/10 text-[var(--color-success)]",
                d.kind === "removed" && "bg-[var(--color-error-bg)]/10 text-[var(--color-error)]",
              )}
            >
              {d.kind === "added" ? "+" : d.kind === "removed" ? "-" : " "} {d.text}
            </div>
          ))}
        </pre>
      )}
    </div>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 计划差异审查组件，用于展示和审批文件变更计划
 */
const PlanDiffReview = memo(function PlanDiffReview({
  queue,
  onApplyAll,
  onRejectOne,
  onDiscardAll,
}: {
  queue: QueuedEdit[];
  onApplyAll: () => Promise<void>;
  onRejectOne: (id: string) => void;
  onDiscardAll: () => void;
}) {
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);
  if (queue.length === 0) return null;

  /**
   * 处理应用所有变更
   */
  const handleApply = useCallback(async () => {
    setBusy(true);
    try {
      await onApplyAll();
    } finally {
      setBusy(false);
    }
  }, [onApplyAll]);

  return (
    <div className="absolute inset-0 z-10 flex flex-col bg-[var(--bg-base-default)]/85 backdrop-blur-xl">
      <div className="flex items-center justify-between border-b border-[var(--border-neutral-l1)] p-3">
        <div className="flex items-center gap-2">
          <span className="font-medium">{t.agentChat.planReviewTitle}</span>
          <Badge variant="outline">{t.agentChat.planReviewPending.replace("{count}", String(queue.length))}</Badge>
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onClick={onDiscardAll} disabled={busy}>
            <Trash2 className="size-3.5" data-icon="inline-start" />
            {t.agentChat.planReviewDiscardAll}
          </Button>
          <Button size="sm" variant="default" onClick={handleApply} disabled={busy}>
            <Check className="size-3.5" data-icon="inline-start" />
            {t.agentChat.planReviewApplyAll.replace("{count}", String(queue.length))}
          </Button>
        </div>
      </div>
      <ul className="flex flex-1 flex-col gap-1.5 overflow-auto p-3">
        {queue.map((q) => (
          <li key={q.id}>
            <PlanRow item={q} onReject={onRejectOne} />
          </li>
        ))}
      </ul>
    </div>
  );
});

export { PlanDiffReview };