import { useEffect, useState, useMemo } from "react";
import { PageContainer, PageHeader, PageHeading, PageTitle, PageDescription } from "@/components/shared/page-layout";
import { EmptyState } from "@/components/shared/state";
import { GitBranchIcon, FolderIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useWorkspaceStore } from "@/stores/workspace";
import { useGit } from "@/features/git/hooks/useGit";
import {
  GitStatusBar,
  GitLogView,
  GitDiffView,
  GitCommitDialog,
  GitRollbackDialog,
} from "@/features/git/components";
import type { RollbackMode } from "@/shared/types";

export function GitPage() {
  const { t } = useI18n();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);

  const activeWorkspace = useMemo(
    () => workspaces.find((ws) => ws.id === activeWorkspaceId) ?? null,
    [workspaces, activeWorkspaceId]
  );
  const workspacePath = activeWorkspace?.path ?? "";

  const {
    gitInstalled,
    gitVersion,
    gitStatus,
    gitLog,
    gitDiff,
    loading,
    checkInstalled,
    install,
    init,
    refresh,
    stageFiles,
    commit,
    rollback,
    loadDiff,
  } = useGit();

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

  if (!activeWorkspace) {
    return (
      <PageContainer>
        <EmptyState
          icon={<FolderIcon />}
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

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GitBranchIcon />
            {t.git.title}
          </PageTitle>
          <PageDescription>
            {activeWorkspace.name} · {workspacePath}
          </PageDescription>
        </PageHeading>
      </PageHeader>

      <GitStatusBar
        branch={gitStatus?.branch ?? null}
        uncommittedCount={uncommittedCount}
        isClean={gitStatus?.is_clean ?? true}
        gitInstalled={gitInstalled}
        gitVersion={gitVersion}
        loading={loading}
        onRefresh={() => workspacePath && void refresh(workspacePath)}
        onCommit={() => setCommitDialogOpen(true)}
        onInstall={() => void install()}
        onInitRepo={handleInitRepo}
      />

      <div className="flex-1 flex gap-4 min-h-0">
        <div className="w-80 flex-shrink-0">
          <GitLogView
            commits={gitLog}
            selectedHash={selectedHash}
            loading={loading}
            onSelectCommit={handleSelectCommit}
            onRollback={handleRollbackClick}
          />
        </div>
        <div className="flex-1 min-w-0">
          <GitDiffView diff={gitDiff} loading={loading} />
        </div>
      </div>

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
