// DiffView - 支持 inline / split 两种模式的差异视图
import { useMemo, useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, lineNumbers, highlightActiveLine, drawSelection } from "@codemirror/view";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { oneDark } from "@codemirror/theme-one-dark";
import type { MergeView } from "@codemirror/merge";
import { cn } from "@/lib/utils";
import { Separator } from "@/components/ui/separator";
import { EmptyState } from "@/components/shared/state";
import { useI18n } from "@/locales/i18n";
import { getLanguageExtension } from "@/features/editor/services/languages";
import type { LineDiffResult, DiffHunk, DiffLine, DiffLineType } from "@/features/version/types";

interface DiffViewProps {
  diffResult: LineDiffResult;
  /** inline=单栏内联，split=并排双栏 */
  mode?: "inline" | "split";
  /** split 模式需要的原始文本 */
  oldText?: string;
  /** split 模式需要的新文本 */
  newText?: string;
  /** 语言标识（用于语法高亮） */
  language?: string;
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

export function DiffView({
  diffResult,
  mode = "inline",
  oldText = "",
  newText = "",
  language = "text",
  showStats = true,
  className,
}: DiffViewProps) {
  const { t } = useI18n();
  const stats = diffResult.stats;
  const totalChanges = stats.lines_added + stats.lines_removed;

  if (mode === "split") {
    return (
      <SplitDiffView
        oldText={oldText}
        newText={newText}
        language={language}
        stats={stats}
        showStats={showStats}
        totalChanges={totalChanges}
        className={className}
      />
    );
  }

  if (totalChanges === 0 && diffResult.hunks.length === 0) {
    return (
      <EmptyState title={t.version.noDiff} className={cn("py-8", className)} />
    );
  }

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {showStats && <DiffStatsView stats={stats} />}
      <div className="flex-1 overflow-auto font-mono text-sm divide-y">
        {diffResult.hunks.map((hunk, hunkIndex) => (
          <DiffHunkView key={hunkIndex} hunk={hunk} />
        ))}
      </div>
    </div>
  );
}

/** 差异统计 */
function DiffStatsView({ stats }: { stats: LineDiffResult["stats"] }) {
  const { t } = useI18n();
  return (
    <>
      <div className="flex gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <span className="size-2 rounded-full bg-[var(--status-success-default)]" />
          +{stats.lines_added} {t.version.additions}
        </span>
        <span className="flex items-center gap-1">
          <span className="size-2 rounded-full bg-destructive" />
          -{stats.lines_removed} {t.version.deletions}
        </span>
        {stats.chars_added > 0 && (
          <span>+{stats.chars_added} {t.version.chars}</span>
        )}
        {stats.chars_removed > 0 && (
          <span>-{stats.chars_removed} {t.version.chars}</span>
        )}
      </div>
      <Separator />
    </>
  );
}

function DiffHunkView({ hunk: hunk }: { hunk: DiffHunk }) {
  return (
    <div>
      <div className="bg-[var(--bg-overlay-l2)] px-2 py-1 text-xs text-muted-foreground sticky top-0">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
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
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.old_number && "invisible"
      )}>
        {line.old_number}
      </div>
      <div className={cn(
        "w-8 text-right select-none opacity-60",
        LINE_NUMBER_COLORS[line.line_type],
        !line.new_number && "invisible"
      )}>
        {line.new_number}
      </div>
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

/** 并排 Diff 视图 - 基于 @codemirror/merge 的 MergeView */
function SplitDiffView({
  oldText,
  newText,
  language,
  stats,
  showStats,
  totalChanges,
  className,
}: {
  oldText: string;
  newText: string;
  language: string;
  stats: LineDiffResult["stats"];
  showStats: boolean;
  totalChanges: number;
  className?: string;
}) {
  const { t } = useI18n();
  const hostRef = useRef<HTMLDivElement>(null);
  const mergeViewRef = useRef<MergeView | null>(null);

  useEffect(() => {
    if (!hostRef.current) return;

    // @codemirror/merge 体积较大（~50KB），仅在 split 模式实际渲染时动态加载。
    // getLanguageExtension 同样动态加载对应 lang-* 包。
    let cancelled = false;
    let mv: MergeView | null = null;

    Promise.all([
      import("@codemirror/merge"),
      getLanguageExtension(language),
    ]).then(([{ MergeView: MergeViewCtor }, langExt]) => {
      if (cancelled || !hostRef.current) return;

      const commonExtensions = [
        lineNumbers(),
        highlightActiveLine(),
        drawSelection(),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        ...langExt,
        oneDark,
        EditorView.lineWrapping,
        EditorState.readOnly.of(true),
        EditorView.theme({
          "&": { height: "100%", fontSize: "13px" },
          ".cm-scroller": { fontFamily: "var(--font-mono, monospace)" },
        }),
      ];

      mv = new MergeViewCtor({
        a: {
          doc: oldText,
          extensions: commonExtensions,
        },
        b: {
          doc: newText,
          extensions: commonExtensions,
        },
        parent: hostRef.current,
        // MergeView 默认 gutter 会同步高亮差异行
      });
      mergeViewRef.current = mv;
    });

    return () => {
      cancelled = true;
      if (mv) mv.destroy();
      mergeViewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [oldText, newText, language]);

  if (totalChanges === 0 && !oldText && !newText) {
    return (
      <EmptyState title={t.version.noDiff} className={cn("py-8", className)} />
    );
  }

  return (
    <div className={cn("flex flex-col gap-3 h-full", className)}>
      {showStats && <DiffStatsView stats={stats} />}
      <div ref={hostRef} className="flex-1 min-h-0 overflow-hidden" />
    </div>
  );
}
