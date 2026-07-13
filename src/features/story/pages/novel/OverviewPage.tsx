import { useState } from "react";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useI18n } from "@/locales/i18n";
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
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import {
  BookOpenIcon,
  EditIcon,
  BookmarkIcon,
  BarChart3Icon,
  InfoIcon,
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
  const [activeTab, setActiveTab] = useState("overview");

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

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1">
        <TabsList>
          <TabsTrigger value="overview">
            <InfoIcon className="size-3" />
            {t.overview.overviewView}
          </TabsTrigger>
          <TabsTrigger value="dashboard">
            <BarChart3Icon className="size-3" />
            {t.overview.dashboardView}
          </TabsTrigger>
          <TabsTrigger value="hooks">
            <BookmarkIcon className="size-3" />
            {t.overview.hookLedgerTitle}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="flex-1">
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.title_label}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-medium">{novel.title}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.genre}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-medium capitalize">{novel.genre}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.status}</CardTitle>
              </CardHeader>
              <CardContent>
                <Badge variant="secondary" className="capitalize">{novel.status}</Badge>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.wordCount}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-medium">{novel.word_count.toLocaleString()}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.chapterCount}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-medium">{novel.chapter_count}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.createdAt}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-medium">{new Date(novel.created_at).toLocaleDateString()}</div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="dashboard" className="flex-1">
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.viz.stats.totalWords}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{novel.word_count.toLocaleString()}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.viz.stats.chapterCount}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{novel.chapter_count}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.overview.avgWordsPerChapter}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {novel.chapter_count > 0
                    ? Math.round(novel.word_count / novel.chapter_count).toLocaleString()
                    : 0}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">{t.viz.stats.hookCount}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{storyState?.hooks?.length || 0}</div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="hooks" className="flex-1">
          <HookLedgerPanel novelId={novel.id} />
        </TabsContent>
      </Tabs>

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