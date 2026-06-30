import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  HistoryIcon,
  GitCompareIcon,
  RotateCwIcon,
} from "lucide-react";
import { PageContainer, PageHeader, PageHeading, PageTitle, PageDescription, PageActions } from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import { useVersion } from "@/features/version/hooks/useVersion";
import { VersionTimeline, DiffView } from "@/features/version/components";
import { useI18n } from "@/shared/i18n";
import type { ChapterVersion } from "@/shared/types";

export function VersionPage({ novelId }: { novelId: string }) {
  const { t } = useI18n();
  const {
    versions,
    diffResult,
    loading,
    loadVersions,
    computeDiff,
    restoreVersion,
  } = useVersion(novelId);

  const [chapterNumber, setChapterNumber] = useState<number>(1);
  const [selectedVersion, setSelectedVersion] = useState<ChapterVersion | null>(null);
  const [compareFrom, setCompareFrom] = useState<ChapterVersion | null>(null);
  const [compareTo, setCompareTo] = useState<ChapterVersion | null>(null);

  useEffect(() => {
    loadVersions(chapterNumber);
  }, [chapterNumber, loadVersions]);

  const handleSelectVersion = (version: ChapterVersion) => {
    setSelectedVersion(version);
  };

  const handleCompare = async (from: ChapterVersion, to: ChapterVersion) => {
    setCompareFrom(from);
    setCompareTo(to);
    await computeDiff(from.id, to.id);
  };

  const handleRestore = async (version: ChapterVersion) => {
    // workspaceId/bookId 复用 novelId（与 routes/index.tsx 中 <VersionPage novelId="default" /> 的 demo 路由一致）
    await restoreVersion(version.id, novelId, novelId);
  };

  return (
    <PageContainer>
      {/* 头部 */}
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <HistoryIcon />
            {t.versionPage.title}
          </PageTitle>
          <PageDescription>
            {t.versionPage.description}
          </PageDescription>
        </PageHeading>
        <PageActions>
          <span className="text-sm text-muted-foreground">{t.versionPage.chapter}</span>
          <Input
            type="number"
            value={chapterNumber}
            onChange={(e) => setChapterNumber(parseInt(e.target.value) || 1)}
            className="w-16 h-8"
            min={1}
          />
        </PageActions>
      </PageHeader>

      {/* 主内容 */}
      <div className="flex-1 flex gap-4">
        {/* 时间线 */}
        <Card className="w-80 flex flex-col">
          <CardHeader className="border-b py-3">
            <CardTitle className="text-sm flex items-center gap-2">
              <HistoryIcon className="size-4" />
              {t.versionPage.versionsCount.replace("{count}", String(versions.length))}
            </CardTitle>
          </CardHeader>
          <CardContent className="flex-1 p-0 overflow-hidden">
            <ScrollArea className="h-full">
              <div className="p-3">
                {loading ? (
                  <LoadingState />
                ) : versions.length === 0 ? (
                  <EmptyState
                    icon={<HistoryIcon />}
                    title={t.versionPage.empty}
                    description={t.versionPage.emptyDescription}
                  />
                ) : (
                  <VersionTimeline
                    versions={versions}
                    selectedVersionId={selectedVersion?.id}
                    onSelectVersion={handleSelectVersion}
                    onCompare={handleCompare}
                  />
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>

        {/* 内容/差异视图 */}
        <Card className="flex-1 flex flex-col">
          <CardHeader className="border-b py-3">
            <CardTitle className="text-sm flex items-center gap-2">
              {compareFrom && compareTo ? (
                <>
                  <GitCompareIcon className="size-4" />
                  {t.versionPage.diffTitle.replace("{from}", String(compareFrom.version_number)).replace("{to}", String(compareTo.version_number))}
                </>
              ) : selectedVersion ? (
                <>
                  <RotateCwIcon className="size-4" />
                  {t.versionPage.versionContent.replace("{version}", String(selectedVersion.version_number))}
                </>
              ) : (
                t.versionPage.selectPrompt
              )}
            </CardTitle>
            {selectedVersion && !compareFrom && (
              <Button
                variant="outline"
                size="sm"
                className="ml-auto"
                onClick={() => handleRestore(selectedVersion)}
              >
                <RotateCwIcon className="size-4" />
                {t.versionPage.restore}
              </Button>
            )}
          </CardHeader>
          <CardContent className="flex-1 p-0 overflow-hidden">
            <ScrollArea className="h-full">
              <div className="p-3">
                {compareFrom && compareTo && diffResult ? (
                  <DiffView diffResult={diffResult} />
                ) : selectedVersion ? (
                  <pre className="whitespace-pre-wrap text-sm font-mono">
                    {selectedVersion.content}
                  </pre>
                ) : (
                  <EmptyState
                    icon={<HistoryIcon />}
                    title={t.versionPage.selectPrompt}
                    description={t.versionPage.selectHint}
                  />
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </PageContainer>
  );
}