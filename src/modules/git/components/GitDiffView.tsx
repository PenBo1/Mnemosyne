import { useState } from "react";
import { cn } from "@/shared/utils";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, LoadingState } from "@/components/shared/state";
import { FileDiffIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import type { Diff } from "@/shared/types";

interface GitDiffViewProps {
  diff: Diff | null;
  loading: boolean;
}

export function GitDiffView({ diff, loading }: GitDiffViewProps) {
  const { t } = useI18n();
  const [activeFileIndex, setActiveFileIndex] = useState(0);

  const files = diff?.files ?? [];
  const activeFile = files[activeFileIndex] ?? null;

  return (
    <Card className="flex flex-col">
      <CardHeader className="border-b py-3">
        <CardTitle className="text-sm flex items-center gap-2">
          <FileDiffIcon className="size-4" />
          {t.git.diff.title}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 p-0 overflow-hidden flex flex-col">
        {loading && files.length === 0 ? (
          <LoadingState label={t.common.loading} />
        ) : files.length === 0 ? (
          <EmptyState title={t.git.diff.empty} />
        ) : (
          <>
            <div className="flex flex-wrap gap-1 border-b border-[var(--border-neutral-l1)] p-2">
              {files.map((file, idx) => (
                <Button
                  key={`${file.path}-${idx}`}
                  variant={idx === activeFileIndex ? "default" : "secondary"}
                  size="xs"
                  className="font-mono"
                  onClick={() => setActiveFileIndex(idx)}
                >
                  {file.path.split("/").pop() || file.path}
                </Button>
              ))}
            </div>
            {activeFile && (
              <div className="flex items-center gap-3 border-b border-[var(--border-neutral-l1)] px-3 py-1.5 text-xs">
                <span className="font-mono text-muted-foreground truncate flex-1">
                  {activeFile.path}
                </span>
                <span className="text-[var(--status-success-default)]">
                  {t.git.diff.additions.replace("{count}", String(activeFile.additions))}
                </span>
                <span className="text-[var(--status-error-default)]">
                  {t.git.diff.deletions.replace("{count}", String(activeFile.deletions))}
                </span>
              </div>
            )}
            <ScrollArea className="flex-1">
              <pre className="text-xs font-mono leading-relaxed">
                {activeFile?.patch
                  ? activeFile.patch.split("\n").map((line, idx) => (
                      <div
                        key={idx}
                        className={cn(
                          "px-3 py-0.5 whitespace-pre-wrap break-all",
                          line.startsWith("+") && !line.startsWith("+++") && "bg-[var(--status-success-surface-l1)]",
                          line.startsWith("-") && !line.startsWith("---") && "bg-[var(--status-error-surface-l1)]",
                          (line.startsWith("@@") || line.startsWith("diff ") || line.startsWith("index ")) && "bg-muted/50 text-muted-foreground"
                        )}
                      >
                        {line}
                      </div>
                    ))
                  : t.git.diff.noDiff}
              </pre>
            </ScrollArea>
          </>
        )}
      </CardContent>
    </Card>
  );
}
