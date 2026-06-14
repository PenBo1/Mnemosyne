import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import { Progress } from "@/components/ui/progress";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  PlusIcon,
  BookOpenIcon,
  MoreVerticalIcon,
  Trash2Icon,
  PenLineIcon,
  EyeIcon,
  FolderOpenIcon,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useNovels } from "@/hooks/useNovels";
import { useWorkspaceStore } from "@/stores/workspace";
import type { Novel } from "@/types";

const GENRES = ["fantasy", "scifi", "romance", "mystery", "thriller", "historical", "modern", "other"];

const STATUS_COLORS: Record<string, string> = {
  draft: "bg-warning/20 text-warning",
  writing: "bg-info/20 text-info",
  completed: "bg-success/20 text-success",
  paused: "bg-muted text-muted-foreground",
};

interface NovelsPageProps {
  onOpenNovel?: (novelId: string, title: string) => void;
}

export function NovelsPage({ onOpenNovel }: NovelsPageProps) {
  const { t } = useI18n();
  const { workspaces, activeWorkspaceId } = useWorkspaceStore();
  const activeWorkspace = workspaces.find((ws) => ws.id === activeWorkspaceId);
  const { novels, loading, create, remove } = useNovels(activeWorkspaceId || undefined);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [genre, setGenre] = useState("fantasy");

  const totalWords = novels.reduce((sum, n) => sum + n.word_count, 0);
  const totalChapters = novels.reduce((sum, n) => sum + n.chapter_count, 0);

  async function handleCreate() {
    try {
      await create(title, genre);
      setDialogOpen(false);
      setTitle("");
      setGenre("fantasy");
    } catch (err) {
      console.error("Failed to create novel:", err);
    }
  }

  async function handleDelete(id: string) {
    try {
      await remove(id);
    } catch (err) {
      console.error("Failed to delete novel:", err);
    }
  }

  if (!activeWorkspaceId) {
    return (
      <div className="flex flex-col gap-6">
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <FolderOpenIcon />
            </EmptyMedia>
            <EmptyTitle>{t.novels.noWorkspace}</EmptyTitle>
            <EmptyDescription>
              {t.novels.noWorkspaceHint}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BookOpenIcon />
            {activeWorkspace?.name || "Novels"}
          </h1>
          <p className="text-sm text-muted-foreground flex items-center gap-2">
            <FolderOpenIcon className="size-3.5" />
            <span className="truncate max-w-[300px]">{activeWorkspace?.path}</span>
            <span className="text-muted-foreground/50">|</span>
            <span>
              {t.novels.novelCount.replace("{count}", String(novels.length))}, {totalChapters} {t.novels.chapters}, {(totalWords / 1000).toFixed(1)}k {t.novels.words}
            </span>
          </p>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <PlusIcon data-icon="inline-start" />
              <span>{t.novels.newNovel}</span>
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t.novels.createTitle}</DialogTitle>
              <DialogDescription>{t.novels.createDesc}</DialogDescription>
            </DialogHeader>
            <FieldGroup>
              <Field>
                <FieldLabel>{t.novels.title_label}</FieldLabel>
                <Input
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={t.novels.titlePlaceholder}
                />
              </Field>
              <Field>
                <FieldLabel>{t.novels.genre}</FieldLabel>
                <Select value={genre} onValueChange={setGenre}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {GENRES.map((g) => (
                      <SelectItem key={g} value={g}>
                        {t.novels.genres[g as keyof typeof t.novels.genres]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            </FieldGroup>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDialogOpen(false)}>
                {t.common.cancel}
              </Button>
              <Button onClick={handleCreate} disabled={!title}>
                {t.common.create}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <Spinner className="size-6" />
        </div>
      ) : novels.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <BookOpenIcon />
            </EmptyMedia>
            <EmptyTitle>{t.novels.empty}</EmptyTitle>
            <EmptyDescription>
              {t.novels.createHint}
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => setDialogOpen(true)}>
              <PlusIcon data-icon="inline-start" />
              {t.novels.newNovel}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {novels.map((novel) => (
            <NovelCard
              key={novel.id}
              novel={novel}
              onDelete={handleDelete}
              onOpen={onOpenNovel}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function NovelCard({
  novel,
  onDelete,
  onOpen,
}: {
  novel: Novel;
  onDelete: (id: string) => void;
  onOpen?: (id: string, title: string) => void;
}) {
  const { t } = useI18n();
  const progress = novel.chapter_count > 0 ? Math.min((novel.word_count / (novel.chapter_count * 3000)) * 100, 100) : 0;

  return (
    <Card className="group transition-shadow hover:shadow-md">
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <CardTitle className="truncate text-lg">{novel.title}</CardTitle>
            <CardDescription className="mt-1 flex items-center gap-2">
              <Badge variant="secondary" className="text-xs">
                {t.novels.genres[novel.genre as keyof typeof t.novels.genres] || novel.genre}
              </Badge>
              <Badge
                variant="outline"
                className={`text-xs ${STATUS_COLORS[novel.status] || ""}`}
              >
                {novel.status}
              </Badge>
            </CardDescription>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" className="opacity-0 group-hover:opacity-100 transition-opacity">
                <MoreVerticalIcon />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {onOpen && (
                <DropdownMenuItem onClick={() => onOpen(novel.id, novel.title)}>
                  <EyeIcon />
                  <span>{t.novels.viewDetail}</span>
                </DropdownMenuItem>
              )}
              <DropdownMenuItem onClick={() => onDelete(novel.id)} className="text-destructive">
                <Trash2Icon />
                <span>{t.novels.delete}</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-3">
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">{t.novels.chapterCount.replace("{count}", String(novel.chapter_count))}</span>
            <span className="text-muted-foreground">{t.novels.wordCountK.replace("{count}", (novel.word_count / 1000).toFixed(1))}</span>
          </div>
          <div className="flex flex-col gap-1">
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>{t.novels.progress}</span>
              <span>{Math.round(progress)}%</span>
            </div>
            <Progress value={progress} className="h-1.5" />
          </div>
          {onOpen && (
            <Button
              variant="outline"
              size="sm"
              className="w-full"
              onClick={() => onOpen(novel.id, novel.title)}
            >
              <PenLineIcon data-icon="inline-start" />
              {t.novels.continueWriting}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
