import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ArrowLeftIcon, FileTextIcon } from "lucide-react";
<<<<<<< Updated upstream
import { useI18n } from "@/lib/i18n";
=======
import { useI18n } from "@/shared/i18n";
import MainAgentPage from "@/pages/MainAgentPage";
>>>>>>> Stashed changes

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
              <div className="text-center py-8 text-muted-foreground text-xs flex flex-col items-center gap-2">
                <FileTextIcon className="size-8 opacity-50" />
                <p>{t.chapterReader.noChapters}</p>
                <p>{t.chapterReader.requestChapter}</p>
              </div>
            ) : (
              chapters.map((ch) => (
                <button
                  key={ch.id}
                  onClick={() => setSelectedChapter(ch)}
                  className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                    selectedChapter?.id === ch.id
                      ? "bg-primary/10 text-primary"
                      : "hover:bg-muted"
                  }`}
                >
                  <div className="flex items-center justify-between gap-1">
                    <span className="truncate">{t.chapterReader.chapterPrefix.replace("{number}", String(ch.number))} {ch.title}</span>
                    <Badge variant="outline" className="text-[10px]">
                      {ch.word_count}
                    </Badge>
                  </div>
                </button>
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
              <div className="prose prose-sm dark:prose-invert max-w-none">
                {selectedChapter.content.split("\n").map((paragraph, i) => (
                  <p key={i}>{paragraph}</p>
                ))}
              </div>
            </ScrollArea>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <FileTextIcon className="size-12 mx-auto mb-3 opacity-30" />
              <p className="text-sm">{t.chapterReader.selectChapter}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
