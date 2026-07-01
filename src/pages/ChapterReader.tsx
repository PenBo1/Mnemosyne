import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ArrowLeftIcon, FileTextIcon } from "lucide-react";
import { EmptyState } from "@/components/shared/state";
import { useI18n } from "@/shared/i18n";

interface Chapter {
  id: string;
  number: number;
  title: string;
  content: string;
  status: string;
  word_count: number;
}

interface ChapterReaderProps {
  novelId: string;
  novelTitle: string;
  onBack: () => void;
}

export function ChapterReader({ novelId, novelTitle, onBack }: ChapterReaderProps) {
  const { t } = useI18n();
  const [chapters] = useState<Chapter[]>([]);
  const [selectedChapter, setSelectedChapter] = useState<Chapter | null>(null);
  // novelId 保留用于未来章节加载功能
  void novelId;

  return (
    <div className="flex h-full">
      {/* 章节侧边栏 */}
      <div className="w-64 border-r flex flex-col">
        <div className="p-3 border-b">
          <Button variant="ghost" size="sm" onClick={onBack} className="w-full justify-start">
            <ArrowLeftIcon data-icon="inline-start" />
            {t.chapterReader.backToNovels}
          </Button>
        </div>
        <div className="p-2 border-b flex flex-col gap-1">
          <h3 className="text-sm font-medium px-2">{novelTitle}</h3>
          <p className="text-xs text-muted-foreground px-2">{t.chapterReader.chapterCount.replace("{count}", String(chapters.length))}</p>
        </div>
        <ScrollArea className="flex-1">
          <div className="p-2 flex flex-col gap-1">
            {chapters.length === 0 ? (
              <EmptyState
                icon={<FileTextIcon className="size-8 opacity-50" />}
                title={t.chapterReader.noChapters}
                description={t.chapterReader.requestChapter}
                className="py-8"
              />
            ) : (
              chapters.map((ch) => (
                <Button
                  key={ch.id}
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelectedChapter(ch)}
                  className={`w-full justify-start font-normal ${
                    selectedChapter?.id === ch.id
                      ? "bg-[var(--bg-overlay-l2)] text-primary"
                      : ""
                  }`}
                >
                  <div className="flex items-center justify-between gap-1">
                    <span className="truncate">{t.chapterReader.chapterPrefix.replace("{number}", String(ch.number))} {ch.title}</span>
                    <Badge variant="outline" className="text-[10px]">
                      {ch.word_count}
                    </Badge>
                  </div>
                </Button>
              ))
            )}
          </div>
        </ScrollArea>
      </div>

      {/* 主内容 */}
      <div className="flex-1 flex flex-col">
        {selectedChapter ? (
          <>
            <div className="p-4 border-b">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold">{t.chapterReader.chapterPrefix.replace("{number}", String(selectedChapter.number))} {selectedChapter.title}</h2>
                  <p className="text-sm text-muted-foreground">{t.chapterReader.wordCount.replace("{count}", String(selectedChapter.word_count))}</p>
                </div>
                <Badge variant={selectedChapter.status === "completed" ? "default" : "secondary"}>
                  {selectedChapter.status}
                </Badge>
              </div>
            </div>
            <ScrollArea className="flex-1 p-6">
              <div className="prose prose-sm max-w-none">
                {selectedChapter.content.split("\n").map((paragraph, i) => (
                  <p key={i}>{paragraph}</p>
                ))}
              </div>
            </ScrollArea>
          </>
        ) : (
          <EmptyState
            icon={<FileTextIcon className="size-12 opacity-30" />}
            title={t.chapterReader.selectChapter}
            className="flex-1"
          />
        )}
      </div>
    </div>
  );
}
