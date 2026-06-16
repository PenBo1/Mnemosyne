import { useState, useEffect } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/lib/i18n";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  BookOpenIcon,
  EditIcon,
  BarChart3Icon,
  SettingsIcon,
} from "lucide-react";
import { ipc } from "@/lib/ipc";
import { WritingDashboard } from "@/components/visualizations";
import type { Novel, ChapterSummary, HookRecord } from "@/types";

interface StoryState {
  current_chapter: number;
  total_words: number;
  hooks: HookRecord[];
  summaries: ChapterSummary[];
  facts: unknown[];
}

export function OverviewPage() {
  const { t } = useI18n();
  const { activeWorkspaceId } = useWorkspaceStore();
  const [novel, setNovel] = useState<Novel | null>(null);
  const [storyState, setStoryState] = useState<StoryState | null>(null);
  const [loading, setLoading] = useState(true);
  const [editOpen, setEditOpen] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editGenre, setEditGenre] = useState("");
  const [view, setView] = useState<"overview" | "dashboard">("overview");

  useEffect(() => {
    if (!activeWorkspaceId) return;
    setLoading(true);
    ipc<Novel[]>("list_novels")
      .then(async (novels) => {
        const found = novels.find((n) => n.workspace_id === activeWorkspaceId);
        setNovel(found || null);
        if (found) {
          try {
            const state = await ipc<StoryState>("story_state_get", { novelId: found.id });
            setStoryState(state);
          } catch {
            setStoryState(null);
          }
        }
      })
      .catch(() => setNovel(null))
      .finally(() => setLoading(false));
  }, [activeWorkspaceId]);

  const handleSave = async () => {
    if (!novel) return;
    try {
      await ipc<Novel>("update_novel", { id: novel.id, title: editTitle, genre: editGenre });
      setNovel({ ...novel, title: editTitle, genre: editGenre });
      setEditOpen(false);
    } catch (err) {
      console.error("Failed to update novel:", err);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        {t.common.loading}
      </div>
    );
  }

  if (!novel) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground gap-4">
        <BookOpenIcon className="size-12 opacity-50" />
        <p>{t.overview.noNovel}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BookOpenIcon />
            {t.overview.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.overview.description}</p>
        </div>
        <div className="flex items-center gap-2">
          <Tabs value={view} onValueChange={(v) => setView(v as "overview" | "dashboard")}>
            <TabsList>
              <TabsTrigger value="overview"><SettingsIcon className="size-3" /> {t.overview.overviewView}</TabsTrigger>
              <TabsTrigger value="dashboard"><BarChart3Icon className="size-3" /> {t.overview.dashboardView}</TabsTrigger>
            </TabsList>
          </Tabs>
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
        </div>
      </div>

      {view === "dashboard" && storyState ? (
        <WritingDashboard
          novel={novel}
          summaries={storyState.summaries}
          hooks={storyState.hooks}
          totalWords={storyState.total_words}
          chapterCount={novel.chapter_count}
        />
      ) : (
        <div className="grid grid-cols-2 gap-4">
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.title_label}</div>
            <div className="text-lg font-medium mt-1">{novel.title}</div>
          </div>
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.genre}</div>
            <div className="text-lg font-medium mt-1 capitalize">{novel.genre}</div>
          </div>
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.status}</div>
            <div className="text-lg font-medium mt-1 capitalize">{novel.status}</div>
          </div>
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.wordCount}</div>
            <div className="text-lg font-medium mt-1">{novel.word_count.toLocaleString()}</div>
          </div>
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.chapterCount}</div>
            <div className="text-lg font-medium mt-1">{novel.chapter_count}</div>
          </div>
          <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{t.overview.createdAt}</div>
            <div className="text-lg font-medium mt-1">{new Date(novel.created_at).toLocaleDateString()}</div>
          </div>
        </div>
      )}

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
    </div>
  );
}
