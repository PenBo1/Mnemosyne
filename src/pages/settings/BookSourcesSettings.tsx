import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel, FieldSeparator } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  MoreVerticalIcon,
  PlusIcon,
  Trash2Icon,
  PencilIcon,
  RefreshCwIcon,
  GlobeIcon,
} from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import { ipc } from "@/infrastructure/api";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import type { BookSource } from "@/shared/types";

export function BookSourcesSettings() {
  const { t } = useI18n();
  const [sources, setSources] = useState<BookSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<BookSource | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);

  // 表单状态
  const [formName, setFormName] = useState("");
  const [formUrl, setFormUrl] = useState("");
  const [formComment, setFormComment] = useState("");
  const [formEnabled, setFormEnabled] = useState(true);
  const [formSearchUrl, setFormSearchUrl] = useState("");
  const [formSearchMethod, setFormSearchMethod] = useState("get");
  const [formSearchResult, setFormSearchResult] = useState("");
  const [formTocItem, setFormTocItem] = useState("");
  const [formChapterTitle, setFormChapterTitle] = useState("");
  const [formChapterContent, setFormChapterContent] = useState("");
  const [formChapterFilter, setFormChapterFilter] = useState("");

  const loadSources = useCallback(async () => {
    setLoading(true);
    try {
      const data = await ipc<BookSource[]>("novel_source_list");
      setSources(data);
    } catch (err) {
      console.error("Failed to load book sources:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSources();
  }, [loadSources]);

  async function handleToggle(source: BookSource) {
    setToggling(source.name);
    try {
      await ipc("novel_source_toggle", { name: source.name, enabled: !source.disabled });
      setSources((prev) =>
        prev.map((s) => (s.name === source.name ? { ...s, disabled: !s.disabled } : s))
      );
    } catch (err) {
      console.error("Failed to toggle source:", err);
    } finally {
      setToggling(null);
    }
  }

  function resetForm() {
    setFormName("");
    setFormUrl("");
    setFormComment("");
    setFormEnabled(true);
    setFormSearchUrl("");
    setFormSearchMethod("get");
    setFormSearchResult("");
    setFormTocItem("");
    setFormChapterTitle("");
    setFormChapterContent("");
    setFormChapterFilter("");
  }

  function openCreateDialog() {
    setEditingSource(null);
    resetForm();
    setDialogOpen(true);
  }

  function openEditDialog(source: BookSource) {
    setEditingSource(source);
    setFormName(source.name);
    setFormUrl(source.url);
    setFormComment(source.comment || "");
    setFormEnabled(!source.disabled);
    setFormSearchUrl(source.search?.url || "");
    setFormSearchMethod(source.search?.method || "get");
    setFormSearchResult(source.search?.result || "");
    setFormTocItem(source.toc?.item || "");
    setFormChapterTitle(source.chapter?.title || "");
    setFormChapterContent(source.chapter?.content || "");
    setFormChapterFilter(source.chapter?.filter_txt || "");
    setDialogOpen(true);
  }

  async function handleSave() {
    const source: BookSource = {
      name: formName,
      url: formUrl,
      comment: formComment,
      disabled: !formEnabled,
      search: formSearchUrl ? {
        disabled: false,
        url: formSearchUrl,
        method: formSearchMethod,
        data: "",
        cookies: "",
        result: formSearchResult,
        book_name: "",
        author: "",
        category: "",
        word_count: "",
        status: "",
        latest_chapter: "",
        last_update_time: "",
        pagination: false,
        next_page: "",
      } : undefined,
      toc: formTocItem ? { base_uri: "", url: "", item: formTocItem, is_desc: false, pagination: false, next_page: "" } : undefined,
      chapter: formChapterContent ? {
        title: formChapterTitle,
        content: formChapterContent,
        paragraph_tag_closed: false,
        paragraph_tag: "",
        filter_txt: formChapterFilter,
        filter_tag: "",
        pagination: false,
        next_page: "",
      } : undefined,
    };

    try {
      if (editingSource) {
        await ipc("novel_source_update", { source });
      } else {
        await ipc("novel_source_add", { source });
      }
      setDialogOpen(false);
      loadSources();
    } catch (err) {
      toast.error(t.common.failedToSave);
      console.error("Failed to save source:", err);
    }
  }

  async function handleDelete(name: string) {
    try {
      await ipc("novel_source_delete", { name });
      setDeleteConfirm(null);
      loadSources();
    } catch (err) {
      toast.error(t.common.failedToDelete);
      console.error("Failed to delete source:", err);
    }
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.bookSources}</PageTitle>
          <PageDescription>{t.settings.bookSourcesDesc}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button variant="outline" size="sm" onClick={loadSources} disabled={loading}>
            <RefreshCwIcon data-icon="inline-start" className={loading ? "animate-spin" : ""} />
          </Button>
          <Button size="sm" onClick={openCreateDialog}>
            <PlusIcon data-icon="inline-start" />
            {t.common.create}
          </Button>
        </PageActions>
      </PageHeader>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : sources.length === 0 ? (
        <EmptyState
          icon={<GlobeIcon className="size-6" />}
          title={t.settings.bookSourcesEmpty}
          description={t.settings.bookSourcesDesc}
        />
      ) : (
        <Card className="py-0 gap-0">
          <CardContent className="divide-y px-0">
            {sources.map((source) => (
              <div key={source.name} className="flex flex-col gap-2 px-4 py-3 transition-colors hover:bg-muted/50">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <GlobeIcon className="size-4 shrink-0" />
                    <span className="text-sm font-medium">{source.name}</span>
                    <Badge variant={source.disabled ? "secondary" : "default"} className="text-xs">
                      {source.disabled ? t.settings.bookSourceDisabled : t.settings.bookSourceEnabled}
                    </Badge>
                    {source.search?.disabled && (
                      <Badge variant="outline" className="text-xs">{t.settings.bookSourceNoSearch}</Badge>
                    )}
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openEditDialog(source)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        onClick={() => setDeleteConfirm(source.name)}
                        className="text-destructive"
                      >
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                <p className="line-clamp-1 text-xs text-muted-foreground">{source.url}</p>
                {source.comment && (
                  <p className="line-clamp-2 text-xs text-muted-foreground">{source.comment}</p>
                )}
                <div className="flex items-center justify-between">
                  <span className="text-xs text-muted-foreground">
                    {source.search?.disabled ? t.settings.bookSourceNoSearch : t.settings.bookSourceSearchable}
                  </span>
                  <Switch
                    checked={!source.disabled}
                    onCheckedChange={() => handleToggle(source)}
                    disabled={toggling === source.name}
                  />
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* 创建/编辑对话框 */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingSource ? t.settings.bookSourceEdit : t.settings.bookSourceCreate}</DialogTitle>
            <DialogDescription>
              {editingSource ? t.settings.bookSourceEditDesc : t.settings.bookSourceCreateDesc}
            </DialogDescription>
          </DialogHeader>
          <ScrollArea className="max-h-[60vh]">
            <FieldGroup>
              <Field>
                <FieldLabel>{t.settings.bookSourceName}</FieldLabel>
                <Input value={formName} onChange={(e) => setFormName(e.target.value)} placeholder={t.settings.bookSourceNamePlaceholder} />
              </Field>
              <Field>
                <FieldLabel>URL</FieldLabel>
                <Input value={formUrl} onChange={(e) => setFormUrl(e.target.value)} placeholder="https://..." />
              </Field>
              <Field>
                <FieldLabel>{t.settings.bookSourceComment}</FieldLabel>
                <Input value={formComment} onChange={(e) => setFormComment(e.target.value)} placeholder={t.settings.bookSourceCommentPlaceholder} />
              </Field>
              <Field>
                <div className="flex items-center justify-between">
                  <FieldLabel>{t.settings.bookSourceEnabled}</FieldLabel>
                  <Switch checked={formEnabled} onCheckedChange={setFormEnabled} />
                </div>
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>{t.settings.bookSourceSearchUrl}</FieldLabel>
                <Input value={formSearchUrl} onChange={(e) => setFormSearchUrl(e.target.value)} placeholder={t.settings.bookSourceSearchUrlPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.settings.bookSourceSearchMethod}</FieldLabel>
                <Input value={formSearchMethod} onChange={(e) => setFormSearchMethod(e.target.value)} placeholder={t.settings.bookSourceSearchMethodPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.settings.bookSourceSearchResult}</FieldLabel>
                <Textarea value={formSearchResult} onChange={(e) => setFormSearchResult(e.target.value)} placeholder={t.settings.bookSourceSearchResultPlaceholder} className="min-h-[60px]" />
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>{t.settings.bookSourceTocItem}</FieldLabel>
                <Input value={formTocItem} onChange={(e) => setFormTocItem(e.target.value)} placeholder={t.settings.bookSourceTocItemPlaceholder} />
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>{t.settings.bookSourceChapterTitle}</FieldLabel>
                <Input value={formChapterTitle} onChange={(e) => setFormChapterTitle(e.target.value)} placeholder={t.settings.bookSourceChapterTitlePlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.settings.bookSourceChapterContent}</FieldLabel>
                <Input value={formChapterContent} onChange={(e) => setFormChapterContent(e.target.value)} placeholder={t.settings.bookSourceChapterContentPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.settings.bookSourceContentFilter}</FieldLabel>
                <Input value={formChapterFilter} onChange={(e) => setFormChapterFilter(e.target.value)} placeholder={t.settings.bookSourceContentFilterPlaceholder} />
              </Field>
            </FieldGroup>
          </ScrollArea>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.common.cancel}
            </Button>
            <Button onClick={handleSave} disabled={!formName.trim()}>
              {editingSource ? t.common.save : t.common.create}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认对话框 */}
      <Dialog open={!!deleteConfirm} onOpenChange={() => setDeleteConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.settings.bookSourceDelete}</DialogTitle>
            <DialogDescription>
              {t.settings.bookSourceDeleteConfirm.replace("{name}", deleteConfirm || "")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
              {t.common.cancel}
            </Button>
            <Button variant="destructive" onClick={() => deleteConfirm && handleDelete(deleteConfirm)}>
              {t.common.delete}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
