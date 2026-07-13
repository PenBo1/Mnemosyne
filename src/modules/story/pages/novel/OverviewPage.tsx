import { useState } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/shared/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import {
  BookOpenIcon,
  EditIcon,
  BookmarkIcon,
} from "lucide-react";
import { useOverview } from "@/features/story/hooks";
import { HookLedgerPanel } from "@/features/story/components/HookLedgerPanel";

export function OverviewPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { novel, storyState, loading, updateNovel } = useOverview(activeWorkspaceId);
  const [editOpen, setEditOpen] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editGenre, setEditGenre] = useState("");

  const handleSave = async () => {
    if (!novel) return;
    await updateNovel(editTitle, editGenre);
    setEditOpen(false);
  };

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  if (!novel) {
    return (
      <PageContainer scrollable={false}>
        <EmptyState icon={<BookOpenIcon />} title={t.overview.noNovel} />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <BookOpenIcon />
            {t.overview.title}
          </PageTitle>
          <PageDescription>{t.overview.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setEditTitle(novel.title);
              setEditGenre(novel.genre);
              setEditOpen(true);
            }}
          >
            <EditIcon className="size-4" />
            {t.overview.editNovel}
          </Button>
        </PageActions>
      </PageHeader>

      {storyState && (
        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.title_label}</div>
            <div className="text-lg font-medium">{novel.title}</div>
          </div>
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.genre}</div>
            <div className="text-lg font-medium capitalize">{novel.genre}</div>
          </div>
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.status}</div>
            <div className="text-lg font-medium capitalize">{novel.status}</div>
          </div>
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.wordCount}</div>
            <div className="text-lg font-medium">{novel.word_count.toLocaleString()}</div>
          </div>
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.chapterCount}</div>
            <div className="text-lg font-medium">{novel.chapter_count}</div>
          </div>
          <div className="flex flex-col gap-1 rounded-[var(--radius-6)] border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.createdAt}</div>
            <div className="text-lg font-medium">{new Date(novel.created_at).toLocaleDateString()}</div>
          </div>
        </div>
      )}

      {/* Hook 账本：展示 + 手动 resolve/defer/reopen */}
      <div className="flex flex-col gap-2">
        <PageHeading>
          <PageTitle>
            <BookmarkIcon />
            {t.overview.hookLedgerTitle}
          </PageTitle>
          <PageDescription>{t.overview.hookLedgerDesc}</PageDescription>
        </PageHeading>
        <HookLedgerPanel novelId={novel.id} />
      </div>

      <Dialog open={editOpen} onOpenChange={setEditOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.overview.editNovel}</DialogTitle>
            <DialogDescription>{t.overview.description}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.overview.title_label}</FieldLabel>
              <Input
                value={editTitle}
                onChange={(e) => setEditTitle(e.target.value)}
                placeholder={t.novels.titlePlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.overview.genre}</FieldLabel>
              <Input
                value={editGenre}
                onChange={(e) => setEditGenre(e.target.value)}
                placeholder={t.novels.genre}
              />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditOpen(false)}>
              {t.common.cancel}
            </Button>
            <Button onClick={handleSave}>{t.common.save}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
