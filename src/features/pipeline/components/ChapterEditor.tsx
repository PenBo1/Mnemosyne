import { useState, useEffect } from "react";
import { useI18n } from "@/locales/i18n";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  FileTextIcon,
  SaveIcon,
  LoaderIcon,
  PlayIcon,
  EyeIcon,
  PencilIcon,
} from "lucide-react";
import type { PipelineChapter } from "@/features/pipeline/types";
import * as pipelineService from "@/features/pipeline/services";

interface ChapterEditorProps {
  bookId: string;
  chapter: PipelineChapter | null;
  onSave?: (content: string) => Promise<void>;
  onRunAudit?: () => Promise<void>;
  loading?: boolean;
}

export function ChapterEditor({
  bookId,
  chapter,
  onSave,
  onRunAudit,
  loading = false,
}: ChapterEditorProps) {
  const { t } = useI18n();
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);

  useEffect(() => {
    if (chapter && chapter.number) {
      setFetching(true);
      pipelineService
        .getChapter(bookId, chapter.number)
        .then((ch) => {
          setContent(ch.content ?? "");
        })
        .catch(() => {
          setContent("");
        })
        .finally(() => setFetching(false));
    }
  }, [bookId, chapter?.number]);

  const handleSave = async () => {
    if (!onSave) return;
    setSaving(true);
    try {
      await onSave(content);
    } finally {
      setSaving(false);
    }
  };

  if (!chapter) {
    return (
      <Card className="p-4 text-center text-muted-foreground">
        {t.pipeline.selectChapterToEdit}
      </Card>
    );
  }

  const wordCount = content.length;

  return (
    <Card className="flex flex-col h-full">
      <div className="flex items-center justify-between gap-2 p-4 border-b">
        <div className="flex items-center gap-2">
          <FileTextIcon className="size-5 text-muted-foreground" />
          <span className="font-medium">
            #{String(chapter.number).padStart(3, "0")} {chapter.title}
          </span>
          <Badge variant="outline">{chapter.status}</Badge>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{wordCount} {t.pipeline.words}</span>
        </div>
      </div>

      <Tabs defaultValue="preview" className="flex-1 flex flex-col">
        <TabsList className="mx-4 mt-2">
          <TabsTrigger value="preview">
            <EyeIcon className="size-3.5" />
            {t.pipeline.preview}
          </TabsTrigger>
          <TabsTrigger value="edit">
            <PencilIcon className="size-3.5" />
            {t.pipeline.edit}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="preview" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            {fetching ? (
              <div className="flex items-center justify-center h-full">
                <LoaderIcon className="size-5 animate-spin text-muted-foreground" />
              </div>
            ) : (
              <div className="prose prose-sm max-w-none whitespace-pre-wrap">
                {content || t.pipeline.noContent}
              </div>
            )}
          </ScrollArea>
        </TabsContent>

        <TabsContent value="edit" className="flex-1 m-0 p-4">
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            className="h-full min-h-[300px] resize-none font-mono text-sm"
            placeholder={t.pipeline.editPlaceholder}
          />
        </TabsContent>
      </Tabs>

      <div className="flex items-center justify-end gap-2 p-4 border-t">
        {onRunAudit && (
          <Button variant="outline" onClick={onRunAudit} disabled={loading}>
            <PlayIcon className="size-4" />
            {t.pipeline.runAudit}
          </Button>
        )}
        {onSave && (
          <Button onClick={handleSave} disabled={saving || loading}>
            {saving ? (
              <LoaderIcon className="size-4 animate-spin" />
            ) : (
              <SaveIcon className="size-4" />
            )}
            {t.pipeline.saveContent}
          </Button>
        )}
      </div>
    </Card>
  );
}