import { useMemo } from "react";
import { cn } from "@/lib/utils";
import type { LineDiffResult, DiffHunk, DiffLine, DiffLineType } from "@/types";

interface DiffViewProps {
  diffResult: LineDiffResult;
  showStats?: boolean;
  className?: string;
}

const LINE_COLORS: Record<DiffLineType, string> = {
  added: "bg-green-100 dark:bg-green-900/30 border-l-2 border-green-500",
  removed: "bg-red-100 dark:bg-red-900/30 border-l-2 border-red-500",
  context: "bg-transparent",
};

const LINE_NUMBER_COLORS: Record<DiffLineType, string> = {
  added: "text-green-600 dark:text-green-400",
  removed: "text-red-600 dark:text-red-400",
  context: "text-muted-foreground",
};

export function DiffView({ diffResult, showStats = true, className }: DiffViewProps) {
  const stats = diffResult.stats;
  const totalChanges = stats.lines_added + stats.lines_removed;

  if (totalChanges === 0 && diffResult.hunks.length === 0) {
    return (
      <div className={cn("text-center text-muted-foreground py-8", className)}>
        No differences found
      </div>
    );
  }

  return (
    <div className={cn("flex flex-col", className)}>
      {showStats && (
        <div className="flex gap-4 mb-3 text-xs text-muted-foreground border-b pb-2">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-green-500" />
            +{stats.lines_added} additions
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-red-500" />
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
      {/* Hunk header */}
      <div className="bg-muted/50 px-2 py-1 text-xs text-muted-foreground sticky top-0">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
      {/* Hunk lines */}
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
      {/* Old line number */}
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.old_number && "invisible"
      )}>
        {line.old_number}
      </div>
      {/* New line number */}
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.new_number && "invisible"
      )}>
        {line.new_number}
      </div>
      {/* Line content */}
      <div className="flex-1 whitespace-pre-wrap break-all pl-1">
        <span className={cn(
          line.line_type === "added" && "text-green-700 dark:text-green-300",
          line.line_type === "removed" && "text-red-700 dark:text-red-300",
        )}>
          {prefix}
        </span>
        {line.content || ""}
      </div>
    </div>
  );
}