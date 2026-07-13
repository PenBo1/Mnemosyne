import { useMemo } from "react";
import { cn } from "@/shared/utils";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/shared/state";
import type { ChapterVersion } from "@/shared/types";

interface VersionTimelineProps {
  versions: ChapterVersion[];
  selectedVersionId?: string | null;
  onSelectVersion: (version: ChapterVersion) => void;
  onCompare?: (fromVersion: ChapterVersion, toVersion: ChapterVersion) => void;
  className?: string;
}

const REVISION_MODE_COLORS: Record<string, string> = {
  auto: "bg-primary",
  polish: "bg-muted-foreground",
  rewrite: "bg-[var(--status-warning-default)]",
  rework: "bg-destructive",
  spot_fix: "bg-[var(--status-warning-default)]",
  manual: "bg-primary",
};

export function VersionTimeline({
  versions,
  selectedVersionId,
  onSelectVersion,
  onCompare,
  className,
}: VersionTimelineProps) {
  const sortedVersions = useMemo(() => {
    return [...versions].sort((a, b) => b.version_number - a.version_number);
  }, [versions]);

  if (versions.length === 0) {
    return (
      <EmptyState title="No versions available" className={cn("py-8", className)} />
    );
  }

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return dateStr;
    }
  };

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      {sortedVersions.map((version, index) => {
        const isSelected = selectedVersionId === version.id;
        const prevVersion = sortedVersions[index + 1];

        return (
          <Card
            key={version.id}
            className={cn(
              "cursor-pointer transition-colors hover:bg-muted/50",
              isSelected && "ring-2 ring-primary bg-primary/5"
            )}
            onClick={() => onSelectVersion(version)}
          >
            <CardContent className="p-3 flex flex-col gap-2">
              {/* 头部 */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-medium">v{version.version_number}</span>
                  <span
                    className={cn(
                      "size-2 rounded-full",
                      REVISION_MODE_COLORS[version.revision_mode] || "bg-muted-foreground"
                    )}
                    title={version.revision_mode}
                  />
                  <span className="text-xs text-muted-foreground capitalize">
                    {version.revision_mode.replace("_", " ")}
                  </span>
                </div>
                <span className="text-xs text-muted-foreground">
                  {formatDate(version.created_at)}
                </span>
              </div>

              {/* 统计 */}
              <div className="flex gap-3 text-xs text-muted-foreground">
                <span>{version.word_count} words</span>
                <span>Chapter {version.chapter_number}</span>
              </div>

              {/* 原因 */}
              {version.revision_reason && (
                <div className="text-xs text-muted-foreground truncate" title={version.revision_reason}>
                  {version.revision_reason}
                </div>
              )}

              {/* 对比按钮 */}
              {onCompare && prevVersion && (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full h-7 text-xs"
                  onClick={(e) => {
                    e.stopPropagation();
                    onCompare(prevVersion, version);
                  }}
                >
                  Compare with v{prevVersion.version_number}
                </Button>
              )}
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
