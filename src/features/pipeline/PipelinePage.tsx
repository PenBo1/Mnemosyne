import { useState } from "react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  BookList,
  SchedulerPanel,
  BookEditor,
  ChapterEditor,
  TruthFileViewer,
} from "./components";
import { usePipeline } from "./hooks";
import { Workflow, PlusIcon, SettingsIcon, FileIcon } from "lucide-react";
import type { PipelineChapter } from "./types";

export default function PipelinePage() {
  const { t } = useI18n();
  const {
    books,
    chapters,
    selectedBookId,
    running,
    createBook,
  } = usePipeline();

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [selectedChapter, setSelectedChapter] = useState<PipelineChapter | null>(null);
  const [editorTab, setEditorTab] = useState<"chapter" | "truth">("chapter");

  const handleCreateBook = async (req: Parameters<typeof createBook>[0]) => {
    await createBook(req);
    setShowCreateDialog(false);
  };

  const handleSelectChapter = (chapter: PipelineChapter) => {
    setSelectedChapter(chapter);
    setEditorTab("chapter");
  };

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <Workflow className="size-5" />
            {t.pipeline.title}
          </PageTitle>
          <PageDescription>{t.pipeline.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={() => setShowCreateDialog(true)} disabled={running}>
            <PlusIcon className="size-4" />
            {t.pipeline.createBook}
          </Button>
        </PageActions>
      </PageHeader>

      <div className="grid gap-4 lg:grid-cols-[300px_1fr] h-[calc(100vh-200px)]">
        <div className="flex flex-col gap-4 overflow-hidden">
          <ScrollArea className="flex-1">
            <div className="flex flex-col gap-2 pr-4">
              <div className="flex items-center justify-between gap-2 mb-2">
                <span className="text-sm font-medium">{t.pipeline.books}</span>
                <span className="text-xs text-muted-foreground">
                  {books.length} {t.pipeline.booksCount}
                </span>
              </div>
              <BookList />
            </div>
          </ScrollArea>

          {selectedBookId && (
            <ScrollArea className="flex-1 min-h-0">
              <div className="flex flex-col gap-2 pr-4">
                <div className="flex items-center justify-between gap-2 mb-2">
                  <span className="text-sm font-medium">{t.pipeline.chapters}</span>
                  <span className="text-xs text-muted-foreground">
                    {chapters.length} {t.pipeline.chaptersCount}
                  </span>
                </div>
                <div className="flex flex-col gap-1">
                  {chapters.map((ch) => (
                    <Button
                      key={ch.number}
                      variant={selectedChapter?.number === ch.number ? "default" : "ghost"}
                      size="sm"
                      className="justify-start"
                      onClick={() => handleSelectChapter(ch)}
                    >
                      <span className="font-mono text-xs">
                        #{String(ch.number).padStart(3, "0")}
                      </span>
                      <span className="truncate">{ch.title}</span>
                    </Button>
                  ))}
                </div>
              </div>
            </ScrollArea>
          )}

          <SchedulerPanel />
        </div>

        <div className="flex flex-col overflow-hidden">
          <Tabs
            value={editorTab}
            onValueChange={(v) => setEditorTab(v as "chapter" | "truth")}
            className="flex-1 flex flex-col"
          >
            <TabsList className="mb-2">
              <TabsTrigger value="chapter">
                <FileIcon className="size-3.5" />
                {t.pipeline.chapterEditor}
              </TabsTrigger>
              <TabsTrigger value="truth">
                <SettingsIcon className="size-3.5" />
                {t.pipeline.truthFiles}
              </TabsTrigger>
            </TabsList>

            <TabsContent value="chapter" className="flex-1 m-0">
              <ChapterEditor
                bookId={selectedBookId ?? ""}
                chapter={selectedChapter}
                loading={running}
              />
            </TabsContent>

            <TabsContent value="truth" className="flex-1 m-0">
              <TruthFileViewer bookId={selectedBookId} />
            </TabsContent>
          </Tabs>
        </div>
      </div>

      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{t.pipeline.createBook}</DialogTitle>
          </DialogHeader>
          <BookEditor
            onSubmit={handleCreateBook}
            onCancel={() => setShowCreateDialog(false)}
            loading={running}
            mode="create"
          />
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}