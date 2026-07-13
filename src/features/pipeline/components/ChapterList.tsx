import { useI18n } from "@/locales/i18n";
import { usePipeline } from "@/features/pipeline/hooks";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { PlayIcon } from "lucide-react";

export function ChapterList() {
  const { t } = useI18n();
  const { chapters, selectedBookId, writeNextChapter, running } = usePipeline();

  if (!selectedBookId) {
    return null;
  }

  return (
    <div className="flex flex-col gap-2">
      <Button
        onClick={() => writeNextChapter(selectedBookId)}
        disabled={running}
        className="w-full"
      >
        <PlayIcon className="size-4" />
        {running ? t.pipeline.running : t.pipeline.writeNextChapter}
      </Button>

      {chapters.length === 0 && (
        <div className="text-muted-foreground p-4 text-sm">
          {t.pipeline.noChapters}
        </div>
      )}

      {chapters.map((ch) => (
        <Card key={ch.number} className="p-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs text-muted-foreground">
                #{String(ch.number).padStart(3, "0")}
              </span>
              <span className="text-sm font-medium">{ch.title}</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">
                {ch.word_count} {t.pipeline.words}
              </span>
              <Badge variant="outline">{ch.status}</Badge>
            </div>
          </div>
        </Card>
      ))}
    </div>
  );
}
