import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  HistoryIcon,
  GitCompareIcon,
  RotateCwIcon,
} from "lucide-react";
import { useVersion } from "@/hooks/useVersion";
import { VersionTimeline } from "@/components/VersionTimeline";
import { DiffView } from "@/components/DiffView";
import type { ChapterVersion } from "@/types";

export function VersionPage({ novelId }: { novelId: string }) {
  const {
    versions,
    diffResult,
    loading,
    loadVersions,
    computeDiff,
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
    // Note: Need workspaceId and bookId for restore
    // For now, just show a message
    console.log("Restore version:", version.id);
  };

  return (
    <div className="flex flex-col gap-6 h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <HistoryIcon />
            Version History
          </h1>
          <p className="text-sm text-muted-foreground">
            View and compare chapter revisions
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Chapter:</span>
          <Input
            type="number"
            value={chapterNumber}
            onChange={(e) => setChapterNumber(parseInt(e.target.value) || 1)}
            className="w-16 h-8"
            min={1}
          />
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex gap-4">
        {/* Timeline */}
        <Card className="w-80 flex flex-col">
          <CardHeader className="border-b py-3">
            <CardTitle className="text-sm flex items-center gap-2">
              <HistoryIcon className="size-4" />
              Versions ({versions.length})
            </CardTitle>
          </CardHeader>
          <CardContent className="flex-1 p-0 overflow-hidden">
            <ScrollArea className="h-full">
              <div className="p-3">
                {loading ? (
                  <div className="flex items-center justify-center py-8">
                    <Spinner className="size-6" />
                  </div>
                ) : versions.length === 0 ? (
                  <Empty>
                    <EmptyHeader>
                      <EmptyMedia variant="icon">
                        <HistoryIcon />
                      </EmptyMedia>
                      <EmptyTitle>No versions</EmptyTitle>
                      <EmptyDescription>No version history for this chapter</EmptyDescription>
                    </EmptyHeader>
                  </Empty>
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

        {/* Content/Diff View */}
        <Card className="flex-1 flex flex-col">
          <CardHeader className="border-b py-3">
            <CardTitle className="text-sm flex items-center gap-2">
              {compareFrom && compareTo ? (
                <>
                  <GitCompareIcon className="size-4" />
                  Diff: v{compareFrom.version_number} → v{compareTo.version_number}
                </>
              ) : selectedVersion ? (
                <>
                  <RotateCwIcon className="size-4" />
                  v{selectedVersion.version_number} Content
                </>
              ) : (
                "Select a version"
              )}
            </CardTitle>
            {selectedVersion && !compareFrom && (
              <Button
                variant="outline"
                size="sm"
                className="ml-auto"
                onClick={() => handleRestore(selectedVersion)}
              >
                <RotateCwIcon className="size-4 mr-1" />
                Restore
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
                  <Empty>
                    <EmptyHeader>
                      <EmptyMedia variant="icon">
                        <HistoryIcon />
                      </EmptyMedia>
                      <EmptyTitle>Select a version</EmptyTitle>
                      <EmptyDescription>
                        Click on a version to view content, or compare two versions
                      </EmptyDescription>
                    </EmptyHeader>
                  </Empty>
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}