import { cn } from "@/shared/utils";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, LoadingState } from "@/components/shared/state";
import { HistoryIcon, RotateCcwIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import type { Commit } from "@/shared/types";

interface GitLogViewProps {
  commits: Commit[];
  selectedHash: string | null;
  loading: boolean;
  onSelectCommit: (hash: string) => void;
  onRollback: (hash: string) => void;
}

function formatRelative(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const seconds = Math.floor(diffMs / 1000);
    if (seconds < 60) return "just now";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months}mo ago`;
    const years = Math.floor(months / 12);
    return `${years}y ago`;
  } catch {
    return dateStr;
  }
}

export function GitLogView({
  commits,
  selectedHash,
  loading,
  onSelectCommit,
  onRollback,
}: GitLogViewProps) {
  const { t } = useI18n();

  return (
    <Card className="flex flex-col">
      <CardHeader className="border-b py-3">
        <CardTitle className="text-sm flex items-center gap-2">
          <HistoryIcon className="size-4" />
          {t.git.log.title}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 p-0 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-2 flex flex-col gap-1">
            {loading && commits.length === 0 ? (
              <LoadingState label={t.common.loading} />
            ) : commits.length === 0 ? (
              <EmptyState title={t.git.log.empty} />
            ) : (
              commits.map((commit) => {
                const isSelected = selectedHash === commit.hash;
                return (
                  <div
                    key={commit.hash}
                    className={cn(
                      "flex flex-col gap-1 rounded-[var(--radius-3)] border border-transparent p-2 cursor-pointer transition-colors hover:bg-muted/50",
                      isSelected && "border-[var(--border-brand-l1)] bg-primary/5"
                    )}
                    onClick={() => onSelectCommit(commit.hash)}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-mono text-xs text-muted-foreground">
                        {commit.short_hash}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {formatRelative(commit.date)}
                      </span>
                    </div>
                    <div className="text-sm line-clamp-2 break-words">
                      {commit.message}
                    </div>
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs text-muted-foreground truncate">
                        {commit.author}
                      </span>
                      {isSelected && (
                        <Button
                          variant="outline"
                          size="xs"
                          onClick={(e) => {
                            e.stopPropagation();
                            onRollback(commit.hash);
                          }}
                        >
                          <RotateCcwIcon />
                          {t.git.log.rollbackToHere}
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
