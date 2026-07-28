/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GitDiffView - Git 差异视图组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, LoadingState } from "@/components/shared/state";
import { FileDiffIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { Diff } from "@/features/git/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface GitDiffViewProps {
  diff: Diff | null;
  loading: boolean;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Git 差异视图，展示文件变更的详细内容
 */
export function GitDiffView({ diff, loading }: GitDiffViewProps) {
  const { t } = useI18n();
  const [activeFileIndex, setActiveFileIndex] = useState(0);

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const files = diff?.files ?? [];
  const activeFile = files[activeFileIndex] ?? null;

  // ── 渲染 ──────────────────────────────────────────────────────────────────

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
            {/* ── 文件标签栏 ────────────────────────────────────────────────── */}
            <div className="flex flex-wrap gap-1 p-2">
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
            <Separator />
            {/* ── 文件信息 ──────────────────────────────────────────────────── */}
            {activeFile && (
              <>
                <div className="flex items-center gap-3 px-3 py-1.5 text-xs">
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
                <Separator />
              </>
            )}
            {/* ── 差异内容 ──────────────────────────────────────────────────── */}
            <ScrollArea className="flex-1">
              {activeFile?.binary ? (
                <div className="px-3 py-4 text-xs text-muted-foreground text-center">
                  二进制文件
                </div>
              ) : (
                <div className="px-3 py-4 text-xs text-muted-foreground text-center">
                  {t.git.diff.noDiff}
                </div>
              )}
            </ScrollArea>
          </>
        )}
      </CardContent>
    </Card>
  );
}