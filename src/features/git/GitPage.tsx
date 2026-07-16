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
  PlusIcon,
  MinusIcon,
  PencilIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useGit } from "@/features/git/hooks/useGit";
import {
  GitCommitDialog,
  GitRollbackDialog,
} from "@/features/git/components";
import type { RollbackMode, Commit, FileChange, FileDiff } from "@/features/git/types";

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

function getStatusIcon(status: string) {
  switch (status) {
    case "added":
      return <PlusIcon className="size-3 text-[var(--status-success-default)]" />;
    case "deleted":
      return <MinusIcon className="size-3 text-destructive" />;
    case "modified":
      return <PencilIcon className="size-3 text-muted-foreground" />;
    case "renamed":
      return <RefreshCwIcon className="size-3 text-muted-foreground" />;
    default:
      return null;
  }
}

export function GitPage() {
  const { t } = useI18n();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);

  const activeWorkspace = useMemo(
    () => workspaces.find((ws) => ws.id === activeWorkspaceId) ?? null,
    [workspaces, activeWorkspaceId]
  );
  // 每个工作区是独立的 git 仓库，Git 操作路径来自活动工作区
  const workspacePath = activeWorkspace?.path ?? "";

  // 细粒度 selector —— 避免 store 任意字段变化都触发整页重渲染
  const gitInstalled = useGit((s) => s.gitInstalled);
  const gitVersion = useGit((s) => s.gitVersion);
  const gitStatus = useGit((s) => s.gitStatus);
  const gitLog = useGit((s) => s.gitLog);
  const gitDiff = useGit((s) => s.gitDiff);
  const loading = useGit((s) => s.loading);
  const checkInstalled = useGit((s) => s.checkInstalled);
  const install = useGit((s) => s.install);
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

  useEffect(() => {
    if (gitInstalled === null) {
      void checkInstalled();
    }
  }, [gitInstalled, checkInstalled]);

  useEffect(() => {
    if (workspacePath && gitInstalled === true) {
      void refresh(workspacePath);
    }
  }, [workspacePath, gitInstalled, refresh]);

  useEffect(() => {
    if (workspacePath && gitInstalled === true && selectedHash) {
      void loadDiff(workspacePath, selectedHash);
    }
  }, [workspacePath, gitInstalled, selectedHash, loadDiff]);

  // 未安装 Git：前置阻断
  if (gitInstalled === false) {
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
          <PageActions>
            <Button onClick={() => void install()} disabled={loading}>
              {loading ? t.git.install.installing : t.git.install.button}
            </Button>
          </PageActions>
        </PageHeader>
      </PageContainer>
    );
  }

  // 无活动工作区：提示用户先选择工作区（每个工作区是独立 git 仓库）
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

  const uncommittedCount =
    (gitStatus?.staged.length ?? 0) +
    (gitStatus?.unstaged.length ?? 0) +
    (gitStatus?.untracked.length ?? 0);

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
    const allPaths = [
      ...gitStatus.unstaged.map((c) => c.path),
      ...gitStatus.untracked,
    ];
    if (allPaths.length === 0) return;
    await stageFiles(workspacePath, allPaths);
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

  const allChanges: FileChange[] = useMemo(() => {
    if (!gitStatus) return [];
    return [
      ...gitStatus.staged,
      ...gitStatus.unstaged,
      ...gitStatus.untracked.map((path) => ({
        path,
        status: "added" as const,
        staged: false,
      })),
    ];
  }, [gitStatus]);

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
          {gitVersion && (
            <Badge variant="outline" className="text-xs">
              {t.git.status.version.replace("{version}", gitVersion)}
            </Badge>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={handleInitRepo}
            disabled={loading || !gitStatus?.branch}
          >
            <GitBranchIcon className="size-4" />
            {t.git.status.initRepo}
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
            disabled={loading || (gitStatus?.is_clean ?? true)}
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
                          key={commit.hash}
                          className={
                            selectedHash === commit.hash
                              ? "bg-[var(--bg-overlay-l3)]"
                              : ""
                          }
                          onClick={() => handleSelectCommit(commit.hash)}
                        >
                          <TableCell>
                            <span className="font-mono text-xs text-muted-foreground">
                              {commit.short_hash}
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
                              {formatRelative(commit.date)}
                            </span>
                          </TableCell>
                          <TableCell className="text-right">
                            {selectedHash === commit.hash && (
                              <Button
                                variant="outline"
                                size="xs"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleRollbackClick(commit.hash);
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
              ) : allChanges.length === 0 ? (
                <EmptyState
                  icon={<FileTextIcon />}
                  title={t.git.noChanges}
                />
              ) : (
                <ScrollArea className="h-full">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="w-8"></TableHead>
                        <TableHead>{t.git.diff.file}</TableHead>
                        <TableHead>{t.git.status.branch}</TableHead>
                        <TableHead className="text-right">{t.git.commit.stagedFiles}</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {allChanges.map((change: FileChange) => (
                        <TableRow key={change.path}>
                          <TableCell>{getStatusIcon(change.status)}</TableCell>
                          <TableCell className="font-mono text-xs truncate max-w-[300px]">
                            {change.path}
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className="text-xs">
                              {change.status}
                            </Badge>
                          </TableCell>
                          <TableCell className="text-right">
                            {change.staged ? (
                              <Badge variant="secondary" className="text-xs">
                                {t.git.commit.stagedFiles}
                              </Badge>
                            ) : (
                              <span className="text-muted-foreground text-xs">
                                —
                              </span>
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
        </TabsContent>
      </Tabs>

      <GitCommitDialog
        open={commitDialogOpen}
        onOpenChange={setCommitDialogOpen}
        stagedFiles={gitStatus?.staged ?? []}
        unstagedPaths={gitStatus?.unstaged.map((c) => c.path) ?? []}
        untrackedPaths={gitStatus?.untracked ?? []}
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