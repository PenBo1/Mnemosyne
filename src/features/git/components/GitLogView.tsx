/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GitLogView - Git 提交历史视图组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, LoadingState } from "@/components/shared/state";
import { HistoryIcon, RotateCcwIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { Commit } from "@/features/git/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface GitLogViewProps {
  commits: Commit[];
  selectedHash: string | null;
  loading: boolean;
  onSelectCommit: (hash: string) => void;
  onRollback: (hash: string) => void;
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 格式化相对时间
 */
function formatRelative(timestamp: number): string {
  try {
    const date = new Date(timestamp * 1000);
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
    return String(timestamp);
  }
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Git 提交历史视图，展示提交记录列表
 */
export function GitLogView({
  commits,
  selectedHash,
  loading,
  onSelectCommit,
  onRollback,
}: GitLogViewProps) {
  const { t } = useI18n();

  // ── 渲染 ──────────────────────────────────────────────────────────────────

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
                const isSelected = selectedHash === commit.id;
                return (
                  <div
                    key={commit.id}
                    className={cn(
                      "flex flex-col gap-1 rounded-[var(--radius-3)] border border-transparent p-2 cursor-pointer transition-colors hover:bg-accent",
                      isSelected && "border-[var(--border-brand-l1)] bg-[var(--bg-overlay-l3)]"
                    )}
                    onClick={() => onSelectCommit(commit.id)}
                  >
                    {/* ── 提交哈希和时间 ──────────────────────────────────────── */}
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-mono text-xs text-muted-foreground">
                        {commit.short_id}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {formatRelative(commit.time)}
                      </span>
                    </div>
                    {/* ── 提交信息 ────────────────────────────────────────────── */}
                    <div className="text-sm line-clamp-2 break-words">
                      {commit.message}
                    </div>
                    {/* ── 作者和回滚按钮 ──────────────────────────────────────── */}
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
                            onRollback(commit.id);
                          }}
                        >
                          <RotateCcwIcon className="size-4" />
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