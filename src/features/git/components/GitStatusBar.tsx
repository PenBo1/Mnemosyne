/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GitStatusBar - Git 状态栏组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { GitBranchIcon, RefreshCwIcon, DownloadIcon, GitCommitIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface GitStatusBarProps {
  branch: string | null;
  uncommittedCount: number;
  isClean: boolean;
  gitInstalled: boolean | null;
  gitVersion: string | null;
  loading: boolean;
  onRefresh: () => void;
  onCommit: () => void;
  onInstall: () => void;
  onInitRepo: () => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Git 状态栏，显示当前分支、未提交数量和操作按钮
 */
export function GitStatusBar({
  branch,
  uncommittedCount,
  isClean,
  gitInstalled,
  gitVersion,
  loading,
  onRefresh,
  onCommit,
  onInstall,
  onInitRepo,
}: GitStatusBarProps) {
  const { t } = useI18n();

  // ── Git 未安装状态 ────────────────────────────────────────────────────────

  if (gitInstalled === false) {
    return (
      <Card size="sm">
        <CardContent className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <GitBranchIcon className="size-4" />
            <span>{t.git.status.notInstalled}</span>
          </div>
          <Button size="sm" onClick={onInstall} disabled={loading}>
            <DownloadIcon />
            {t.git.status.install}
          </Button>
        </CardContent>
      </Card>
    );
  }

  // ── 正常状态渲染 ──────────────────────────────────────────────────────────

  return (
    <Card size="sm">
      <CardContent className="flex items-center justify-between gap-3">
        {/* ── 分支和状态信息 ────────────────────────────────────────────────── */}
        <div className="flex items-center gap-3 text-sm">
          <div className="flex items-center gap-1.5">
            <GitBranchIcon className="size-4 text-muted-foreground" />
            <span className="font-medium">{branch ?? "—"}</span>
          </div>
          <span className="text-muted-foreground">·</span>
          {isClean ? (
            <span className="text-muted-foreground">{t.git.status.clean}</span>
          ) : (
            <span className="text-[var(--status-warning-default)]">
              {t.git.status.uncommittedCount.replace("{count}", String(uncommittedCount))}
            </span>
          )}
          {gitVersion && (
            <span className="text-xs text-muted-foreground">
              {t.git.status.version.replace("{version}", gitVersion)}
            </span>
          )}
        </div>
        {/* ── 操作按钮 ──────────────────────────────────────────────────────── */}
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={onInitRepo}
            disabled={loading || !branch}
          >
            <GitBranchIcon />
            {t.git.status.initRepo}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            disabled={loading}
          >
            <RefreshCwIcon className={cn(loading && "animate-spin")} />
            {t.git.status.refresh}
          </Button>
          <Button size="sm" onClick={onCommit} disabled={loading || isClean}>
            <GitCommitIcon className="size-4" />
            {t.git.commit.submit}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}