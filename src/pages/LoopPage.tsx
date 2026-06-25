import { useState, useCallback } from "react";
import { useI18n } from "@/lib/i18n";
import { useLoopEngine } from "@/hooks/useLoopEngine";
import { useWorkspaceStore } from "@/stores/workspace";
import { LoopDashboard } from "@/components/loop/LoopDashboard";
import { LoopRunLog } from "@/components/loop/LoopRunLog";
import { LoopPatternEditor } from "@/components/loop/LoopPatternEditor";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import type { CreateLoopStateRequest } from "@/types";

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
      <div className="flex items-center justify-center h-full text-muted-foreground">
        {t.loop?.common?.selectNovel ?? "Select a novel first"}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <h1 className="text-lg font-semibold">{t.loop?.title ?? "Loop Engineering"}</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCreateOpen(true)}
          >
            <Plus className="h-4 w-4 mr-1" />
            {t.loop?.newLoop ?? "New Loop"}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {loading ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Loading...
          </div>
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
      </div>

      <LoopPatternEditor
        open={createOpen}
        onOpenChange={setCreateOpen}
        patterns={patterns}
        onSubmit={handleCreate}
      />
    </div>
  );
}
