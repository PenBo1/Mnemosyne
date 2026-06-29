import { useState, useCallback } from "react";
import { useI18n } from "@/shared/i18n";
import { useLoopEngine } from "@/features/loop/hooks/useLoopEngine";
import { useWorkspaceStore } from "@/stores/workspace";
import { LoopDashboard } from "@/features/loop/components/LoopDashboard";
import { LoopRunLog } from "@/features/loop/components/LoopRunLog";
import { LoopPatternEditor } from "@/features/loop/components/LoopPatternEditor";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import type { CreateLoopStateRequest } from "@/shared/types";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";

export default function LoopPage() {
  const { t } = useI18n();
  const activeNovelId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const {
    states,
    patterns,
    runLogs,
    loading,
    createState,
    deleteState,
    runCycle,
    pauseLoop,
    resumeLoop,
    loadRunLogs,
  } = useLoopEngine(activeNovelId);

  const [createOpen, setCreateOpen] = useState(false);
  const [selectedStateId, setSelectedStateId] = useState<string | null>(null);

  const handleCreate = useCallback(
    async (req: CreateLoopStateRequest) => {
      await createState(req);
      setCreateOpen(false);
    },
    [createState]
  );

  const handleRun = useCallback(
    async (stateId: string) => {
      await runCycle(stateId);
    },
    [runCycle]
  );

  const handleSelectState = useCallback(
    async (stateId: string) => {
      setSelectedStateId(stateId);
      await loadRunLogs(stateId);
    },
    [loadRunLogs]
  );

  if (!activeNovelId) {
    return (
      <PageContainer>
        <EmptyState
          title={t.loop?.common?.selectNovel ?? "Select a novel first"}
        />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.loop?.title ?? "Loop Engineering"}</PageTitle>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCreateOpen(true)}
          >
            <Plus data-icon="inline-start" />
            {t.loop?.newLoop ?? "New Loop"}
          </Button>
        </PageActions>
      </PageHeader>

      {loading ? (
        <LoadingState />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <div className="lg:col-span-2">
            <LoopDashboard
              states={states}
              patterns={patterns}
              onRun={handleRun}
              onPause={pauseLoop}
              onResume={resumeLoop}
              onDelete={deleteState}
              onSelect={handleSelectState}
              selectedStateId={selectedStateId}
            />
          </div>
          <div>
            <LoopRunLog
              logs={runLogs}
              selectedStateId={selectedStateId}
            />
          </div>
        </div>
      )}

      <LoopPatternEditor
        open={createOpen}
        onOpenChange={setCreateOpen}
        patterns={patterns}
        onSubmit={handleCreate}
      />
    </PageContainer>
  );
}
