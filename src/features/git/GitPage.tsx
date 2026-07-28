/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GitPage - Git 版本控制主页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { EmptyState, LoadingState } from "@/components/shared/state";
import {
  GitBranchIcon,
  FolderIcon,
  HistoryIcon,
  FileTextIcon,
  RefreshCwIcon,
  GitCommitIcon,
  RotateCcwIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useGit } from "@/features/git/hooks/useGit";
import {
  GitCommitDialog,
  GitRollbackDialog,
  ChangesPanel,
} from "@/features/git/components";
import type { RollbackMode, Commit, FileDiff } from "@/features/git/types";

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
 * Git 版本控制主页面，展示提交历史和文件变更
 */
export function GitPage() {
  const { t } = useI18n();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);

  const activeWorkspace = useMemo(
    () => workspaces.find((ws) => ws.id === activeWorkspaceId) ?? null,
    [workspaces, activeWorkspaceId]
  );
  // 每个工作区是独立的 git 仓库
  const workspacePath = activeWorkspace?.path ?? "";

  // ── 状态管理 ──────────────────────────────────────────────────────────────

  // 细粒度 selector
  const gitStatus = useGit((s) => s.gitStatus);
  const gitLog = useGit((s) => s.gitLog);
  const gitDiff = useGit((s) => s.gitDiff);
  const loading = useGit((s) => s.loading);
  const init = useGit((s) => s.init);
  const refresh = useGit((s) => s.refresh);
  const stageFiles = useGit((s) => s.stageFiles);
  const commit = useGit((s) => s.commit);
  const rollback = useGit((s) => s.rollback);
  const loadDiff = useGit((s) => s.loadDiff);

  const [activeTab, setActiveTab] = useState<"history" | "changes">("history");
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [commitDialogOpen, setCommitDialogOpen] = useState(false);
  const [rollbackDialogOpen, setRollbackDialogOpen] = useState(false);
  const [rollbackTarget, setRollbackTarget] = useState<string | null>(null);

  // ── 初始化和刷新 ──────────────────────────────────────────────────────────

  useEffect(() => {
    let cancelled = false;
    if (workspacePath) {
      refresh(workspacePath).catch((err: Error) => {
        if (!cancelled) console.error("[GitPage] refresh failed", err);
      });
    }
    return () => {
      cancelled = true;
    };
  }, [workspacePath, refresh]);

  useEffect(() => {
    let cancelled = false;
    if (workspacePath && activeTab === "history" && selectedHash) {
      loadDiff(workspacePath, false, selectedHash).catch((err: Error) => {
        if (!cancelled) console.error("[GitPage] loadDiff failed", err);
      });
    }
    return () => {
      cancelled = true;
    };
  }, [workspacePath, activeTab, selectedHash, loadDiff]);

  // 变更页内联 diff
  useEffect(() => {
    let cancelled = false;
    if (workspacePath && activeTab === "changes") {
      loadDiff(workspacePath, false).catch((err: Error) => {
        if (!cancelled) console.error("[GitPage] loadDiff failed", err);
      });
    }
    return () => {
      cancelled = true;
    };
  }, [workspacePath, activeTab, loadDiff]);

  // ── 无活动工作区 ──────────────────────────────────────────────────────────

  if (!workspacePath) {
    return (
      <PageContainer>
        <PageHeader>
          <PageHeading>
            <PageTitle>
              <GitBranchIcon className="size-4" />
              {t.git.title}
            </PageTitle>
            <PageDescription>{t.git.notAvailable}</PageDescription>
          </PageHeading>
        </PageHeader>
        <EmptyState
          icon={<FolderIcon className="size-6" />}
          title={t.novels.noWorkspace}
          description={t.novels.noWorkspaceHint}
        />
      </PageContainer>
    );
  }

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const uncommittedCount =
    (gitStatus?.staged ?? 0) +
    (gitStatus?.unstaged ?? 0);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  const handleSelectCommit = (hash: string) => {
    setSelectedHash(hash);
  };

  const handleRollbackClick = (hash: string) => {
    setRollbackTarget(hash);
    setRollbackDialogOpen(true);
  };

  const handleConfirmRollback = async (mode: RollbackMode) => {
    if (!workspacePath || !rollbackTarget) return;
    const ok = await rollback(workspacePath, rollbackTarget, mode);
    if (ok) {
      setRollbackDialogOpen(false);
      setRollbackTarget(null);
      setSelectedHash(null);
    }
  };

  const handleStageAll = async () => {
    if (!workspacePath || !gitStatus) return;
    const allPaths = gitStatus.files.map((f) => f.path);
    if (allPaths.length === 0) return;
    await stageFiles(workspacePath, allPaths);
  };

  const handleStage = async (path: string) => {
    if (!workspacePath) return;
    await stageFiles(workspacePath, [path]);
  };

  const handleCommit = async (message: string) => {
    if (!workspacePath) return;
    const hash = await commit(workspacePath, message);
    if (hash !== null) {
      setCommitDialogOpen(false);
    }
  };

  const handleInitRepo = async () => {
    if (!workspacePath) return;
    const ok = await init(workspacePath);
    if (ok) {
      void refresh(workspacePath);
    }
  };

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GitBranchIcon className="size-4" />
            {t.git.title}
          </PageTitle>
          <PageDescription>
            {gitStatus?.branch ?? "—"} · {workspacePath}
          </PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            onClick={handleInitRepo}
            disabled={loading}
          >
            <GitBranchIcon className="size-4" />
            初始化仓库
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => workspacePath && void refresh(workspacePath)}
            disabled={loading}
          >
            <RefreshCwIcon className={cn("size-4", loading && "animate-spin")} />
            {t.git.status.refresh}
          </Button>
          <Button
            size="sm"
            onClick={() => setCommitDialogOpen(true)}
            disabled={loading || uncommittedCount === 0}
          >
            <GitCommitIcon className="size-4" />
            {t.git.commit.submit}
          </Button>
        </PageActions>
      </PageHeader>

      <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as "history" | "changes")}
        className="flex-1 flex flex-col gap-4"
      >
        <TabsList>
          <TabsTrigger value="history">
            <HistoryIcon className="size-4" />
            {t.git.history}
            {gitLog.length > 0 && (
              <Badge variant="outline" className="size-5 justify-center text-xs">
                {gitLog.length}
              </Badge>
            )}
          </TabsTrigger>
          <TabsTrigger value="changes">
            <FileTextIcon className="size-4" />
            {t.git.changes}
            {uncommittedCount > 0 && (
              <Badge variant="outline" className="size-5 justify-center text-xs">
                {uncommittedCount}
              </Badge>
            )}
          </TabsTrigger>
        </TabsList>

        {/* ── 历史记录 ──────────────────────────────────────────────────────── */}
        <TabsContent value="history" className="flex-1 mt-0">
          <Card className="flex flex-col h-full">
            <CardHeader className="border-b py-3">
              <CardTitle className="text-sm flex items-center gap-2">
                <HistoryIcon className="size-4" />
                {t.git.log.title}
              </CardTitle>
            </CardHeader>
            <CardContent className="flex-1 p-0 overflow-hidden">
              {loading && gitLog.length === 0 ? (
                <LoadingState label={t.common.loading} />
              ) : gitLog.length === 0 ? (
                <EmptyState
                  icon={<HistoryIcon />}
                  title={t.git.log.empty}
                />
              ) : (
                <ScrollArea className="h-full">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t.git.log.shortHash}</TableHead>
                        <TableHead>{t.git.log.message}</TableHead>
                        <TableHead>{t.git.log.author}</TableHead>
                        <TableHead>{t.git.log.date}</TableHead>
                        <TableHead className="text-right">{t.common.edit}</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {gitLog.map((commit: Commit) => (
                        <TableRow
                          key={commit.id}
                          className={
                            selectedHash === commit.id
                              ? "bg-[var(--bg-overlay-l3)]"
                              : ""
                          }
                          onClick={() => handleSelectCommit(commit.id)}
                        >
                          <TableCell>
                            <span className="font-mono text-xs text-muted-foreground">
                              {commit.short_id}
                            </span>
                          </TableCell>
                          <TableCell>
                            <span className="line-clamp-2">{commit.message}</span>
                          </TableCell>
                          <TableCell>
                            <span className="text-muted-foreground truncate">
                              {commit.author}
                            </span>
                          </TableCell>
                          <TableCell>
                            <span className="text-xs text-muted-foreground">
                              {formatRelative(commit.time)}
                            </span>
                          </TableCell>
                          <TableCell className="text-right">
                            {selectedHash === commit.id && (
                              <Button
                                variant="outline"
                                size="xs"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleRollbackClick(commit.id);
                                }}
                              >
                                <RotateCcwIcon />
                                {t.git.log.rollbackToHere}
                              </Button>
                            )}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </ScrollArea>
              )}
            </CardContent>
          </Card>

          {selectedHash && gitDiff && (
            <Card className="mt-4">
              <CardHeader className="border-b py-3">
                <CardTitle className="text-sm flex items-center gap-2">
                  {t.git.diff.title}
                </CardTitle>
              </CardHeader>
              <CardContent className="p-0">
                <ScrollArea className="h-48">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t.git.diff.file}</TableHead>
                        <TableHead className="text-right">
                          {t.git.diff.additions.replace("{count}", "")}
                        </TableHead>
                        <TableHead className="text-right">
                          {t.git.diff.deletions.replace("{count}", "")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {gitDiff.files.map((file: FileDiff) => (
                        <TableRow key={file.path}>
                          <TableCell className="font-mono text-xs truncate max-w-[300px]">
                            {file.path}
                          </TableCell>
                          <TableCell className="text-right text-[var(--status-success-default)]">
                            +{file.additions}
                          </TableCell>
                          <TableCell className="text-right text-destructive">
                            -{file.deletions}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </ScrollArea>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* ── 文件变更 ──────────────────────────────────────────────────────── */}
        <TabsContent value="changes" className="flex-1 mt-0">
          <Card className="flex flex-col h-full">
            <CardHeader className="border-b py-3">
              <CardTitle className="text-sm flex items-center gap-2">
                <FileTextIcon className="size-4" />
                {t.git.changes}
              </CardTitle>
            </CardHeader>
            <CardContent className="flex-1 p-0 overflow-hidden">
              {loading ? (
                <LoadingState label={t.common.loading} />
              ) : !gitStatus || gitStatus.files.length === 0 ? (
                <EmptyState
                  icon={<FileTextIcon />}
                  title={t.git.noChanges}
                />
              ) : (
                <ScrollArea className="h-full">
                  <ChangesPanel
                    status={gitStatus}
                    diff={gitDiff}
                    onStage={(path: string) => void handleStage(path)}
                    onStageAll={() => void handleStageAll()}
                  />
                </ScrollArea>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <GitCommitDialog
        open={commitDialogOpen}
        onOpenChange={setCommitDialogOpen}
        stagedFiles={gitStatus?.files ?? []}
        unstagedPaths={[]}
        untrackedPaths={[]}
        loading={loading}
        onStageAll={handleStageAll}
        onCommit={handleCommit}
      />

      <GitRollbackDialog
        open={rollbackDialogOpen}
        onOpenChange={setRollbackDialogOpen}
        commitHash={rollbackTarget}
        loading={loading}
        onConfirm={handleConfirmRollback}
      />
    </PageContainer>
  );
}