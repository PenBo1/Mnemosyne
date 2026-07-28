/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LoopPage - 循环任务管理页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback } from "react";
import { useI18n } from "@/locales/i18n";
import { useLoopEngine } from "@/features/loop/hooks/useLoopEngine";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { LoopDashboard } from "@/features/loop/components/LoopDashboard";
import { LoopRunLog } from "@/features/loop/components/LoopRunLog";
import { LoopPatternEditor } from "@/features/loop/components/LoopPatternEditor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { CpuIcon, PlusIcon } from "lucide-react";
import type { CreateLoopStateRequest } from "@/features/loop/types";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
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
  const [activeTab, setActiveTab] = useState("dashboard");

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
          icon={<CpuIcon />}
          title={t.loop.common.selectNovel}
        />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <CpuIcon className="size-4" />
            {t.loop.title}
          </PageTitle>
          <PageDescription>{t.loop.common.noLoops}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button size="sm" onClick={() => setCreateOpen(true)}>
            <PlusIcon data-icon="inline-start" />
            {t.loop.newLoop}
          </Button>
        </PageActions>
      </PageHeader>

      {loading ? (
        <LoadingState />
      ) : (
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList>
            <TabsTrigger value="dashboard">{t.loop.dashboard}</TabsTrigger>
            <TabsTrigger value="logs">{t.loop.runLogs}</TabsTrigger>
          </TabsList>

          <TabsContent value="dashboard">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              <div className="lg:col-span-2">
                <Card>
                  <CardHeader>
                    <CardTitle className="trae-card-eyebrow">
                      {t.loop.dashboard}
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
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
                  </CardContent>
                </Card>
              </div>
              <div>
                <Card>
                  <CardHeader>
                    <CardTitle className="trae-card-eyebrow">
                      {t.loop.runLogs}
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <LoopRunLog
                      logs={runLogs}
                      selectedStateId={selectedStateId}
                    />
                  </CardContent>
                </Card>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="logs">
            <Card>
              <CardHeader>
                <CardTitle className="trae-card-eyebrow">
                  {t.loop.runLogs}
                </CardTitle>
              </CardHeader>
              <CardContent>
                <LoopRunLog
                  logs={runLogs}
                  selectedStateId={selectedStateId}
                />
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
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
