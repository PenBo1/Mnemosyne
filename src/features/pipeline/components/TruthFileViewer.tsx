import { useState, useEffect } from "react";
import { useI18n } from "@/locales/i18n";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import {
  FileIcon,
  DatabaseIcon,
  UsersIcon,
  BookIcon,
  ClockIcon,
  MapIcon,
  RulerIcon,
  FrameIcon,
} from "lucide-react";
import type { TruthFileKind } from "@/features/pipeline/types";
import * as pipelineService from "@/features/pipeline/services";

const TRUTH_FILE_KINDS: { kind: TruthFileKind; icon: React.ReactNode; labelKey: "currentState" | "pendingHooks" | "chapterSummaries" | "volumeMap" | "roles" | "bookRules" | "storyFrame" }[] = [
  { kind: "current_state", icon: <DatabaseIcon className="size-3.5" />, labelKey: "currentState" },
  { kind: "pending_hooks", icon: <ClockIcon className="size-3.5" />, labelKey: "pendingHooks" },
  { kind: "chapter_summaries", icon: <BookIcon className="size-3.5" />, labelKey: "chapterSummaries" },
  { kind: "volume_map", icon: <MapIcon className="size-3.5" />, labelKey: "volumeMap" },
  { kind: "roles", icon: <UsersIcon className="size-3.5" />, labelKey: "roles" },
  { kind: "book_rules", icon: <RulerIcon className="size-3.5" />, labelKey: "bookRules" },
  { kind: "story_frame", icon: <FrameIcon className="size-3.5" />, labelKey: "storyFrame" },
];

interface TruthFileViewerProps {
  bookId: string | null;
}

export function TruthFileViewer({ bookId }: TruthFileViewerProps) {
  const { t } = useI18n();
  const [activeKind, setActiveKind] = useState<TruthFileKind>("current_state");
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!bookId) {
      setContent(null);
      return;
    }
    setLoading(true);
    setError(null);
    pipelineService
      .readTruthFile(bookId, activeKind)
      .then((text) => {
        setContent(text);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : "Failed to load truth file");
      })
      .finally(() => setLoading(false));
  }, [bookId, activeKind]);

  if (!bookId) {
    return (
      <Card className="p-4 text-center text-muted-foreground">
        {t.pipeline.selectBookToViewTruth}
      </Card>
    );
  }

  return (
    <Card className="flex flex-col h-full">
      <div className="flex items-center gap-2 p-4 border-b">
        <FileIcon className="size-5 text-muted-foreground" />
        <span className="font-medium">{t.pipeline.truthFiles}</span>
        <Badge variant="secondary">{activeKind}</Badge>
      </div>

      <Tabs
        value={activeKind}
        onValueChange={(v) => setActiveKind(v as TruthFileKind)}
        className="flex-1 flex flex-col"
      >
        <TabsList className="mx-4 mt-2 flex-wrap gap-1">
          {TRUTH_FILE_KINDS.map(({ kind, icon, labelKey }) => (
            <TabsTrigger key={kind} value={kind} className="gap-1">
              {icon}
              {t.pipeline.truthFileLabels[labelKey]}
            </TabsTrigger>
          ))}
        </TabsList>

        <TabsContent value={activeKind} className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            {loading ? (
              <div className="flex flex-col gap-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
                <Skeleton className="h-20 w-full" />
              </div>
            ) : error ? (
              <div className="text-destructive text-sm">{error}</div>
            ) : content ? (
              <div className="prose prose-sm max-w-none whitespace-pre-wrap font-mono text-xs">
                {content}
              </div>
            ) : (
              <div className="text-muted-foreground text-sm">
                {t.pipeline.truthFileEmpty}
              </div>
            )}
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </Card>
  );
}