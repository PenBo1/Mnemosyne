/**
 * ═══════════════════════════════════════════════════════════════════════════
 * BookList - 书籍列表组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useI18n } from "@/locales/i18n";
import { usePipeline } from "@/features/pipeline/hooks";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { BookOpenIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export function BookList() {
  const { t } = useI18n();
  const { books, selectedBookId, selectBook, loading } = usePipeline();

  if (loading && books.length === 0) {
    return <div className="text-muted-foreground p-4">{t.common.loading}</div>;
  }

  if (books.length === 0) {
    return (
      <div className="text-muted-foreground p-4 text-sm">
        {t.pipeline.empty}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {books.map((book) => (
        <Card
          key={book.id}
          className={cn(
            "cursor-pointer p-4 transition-colors hover:bg-accent",
            selectedBookId === book.id && "border-primary bg-accent",
          )}
          onClick={() => selectBook(book.id)}
        >
          <div className="flex items-start justify-between gap-2">
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                <BookOpenIcon className="size-4 text-muted-foreground" />
                <span className="font-medium">{book.title}</span>
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>{book.genre}</span>
                <span>·</span>
                <span>{book.target_chapters} chapters</span>
                <span>·</span>
                <span>{book.chapter_word_count} words/ch</span>
              </div>
            </div>
            <Badge variant="secondary">{book.status}</Badge>
          </div>
        </Card>
      ))}
    </div>
  );
}
