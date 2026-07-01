import { useMemo } from "react";
import { cn } from "@/shared/utils";
import { EmptyState } from "@/components/shared/state";
import type { LineDiffResult, DiffHunk, DiffLine, DiffLineType } from "@/shared/types";

interface DiffViewProps {
  diffResult: LineDiffResult;
  showStats?: boolean;
  className?: string;
}

const LINE_COLORS: Record<DiffLineType, string> = {
  added: "bg-[var(--status-success-default)]/10 border-l-2 border-[var(--status-success-default)]",
  removed: "bg-destructive/10 border-l-2 border-destructive",
  context: "bg-transparent",
};

const LINE_NUMBER_COLORS: Record<DiffLineType, string> = {
  added: "text-[var(--status-success-default)]",
  removed: "text-destructive",
  context: "text-muted-foreground",
};

export function DiffView({ diffResult, showStats = true, className }: DiffViewProps) {
  const stats = diffResult.stats;
  const totalChanges = stats.lines_added + stats.lines_removed;

  if (totalChanges === 0 && diffResult.hunks.length === 0) {
    return (
      <EmptyState title="No differences found" className={cn("py-8", className)} />
    );
  }

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {showStats && (
        <div className="flex gap-4 text-xs text-muted-foreground border-b pb-2">
          <span className="flex items-center gap-1">
            <span className="size-2 rounded-full bg-[var(--status-success-default)]" />
            +{stats.lines_added} additions
          </span>
          <span className="flex items-center gap-1">
            <span className="size-2 rounded-full bg-destructive" />
            -{stats.lines_removed} deletions
          </span>
          {stats.chars_added > 0 && (
            <span>+{stats.chars_added} chars</span>
          )}
          {stats.chars_removed > 0 && (
            <span>-{stats.chars_removed} chars</span>
          )}
        </div>
      )}
      <div className="flex-1 overflow-auto font-mono text-sm">
        {diffResult.hunks.map((hunk, hunkIndex) => (
          <DiffHunkView key={hunkIndex} hunk={hunk} />
        ))}
      </div>
    </div>
  );
}

function DiffHunkView({ hunk: hunk }: { hunk: DiffHunk }) {
  return (
    <div className="border-b last:border-b-0">
      {/* Hunk 头部 */}
      <div className="bg-[var(--bg-overlay-l2)] px-2 py-1 text-xs text-muted-foreground sticky top-0">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
      {/* Hunk 行 */}
      <div className="divide-y divide-transparent">
        {hunk.lines.map((line, lineIndex) => (
          <DiffLineView key={lineIndex} line={line} />
        ))}
      </div>
    </div>
  );
}

function DiffLineView({ line }: { line: DiffLine }) {
  const prefix = useMemo(() => {
    switch (line.line_type) {
      case "added": return "+";
      case "removed": return "-";
      case "context": return " ";
    }
  }, [line.line_type]);

  return (
    <div className={cn("flex gap-2", LINE_COLORS[line.line_type])}>
      {/* 旧行号 */}
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.old_number && "invisible"
      )}>
        {line.old_number}
      </div>
      {/* 新行号 */}
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.new_number && "invisible"
      )}>
        {line.new_number}
      </div>
      {/* 行内容 */}
      <div className="flex-1 whitespace-pre-wrap break-all pl-1">
        <span className={cn(
          line.line_type === "added" && "text-[var(--status-success-default)]",
          line.line_type === "removed" && "text-destructive",
        )}>
          {prefix}
        </span>
        {line.content || ""}
      </div>
    </div>
  );
}
