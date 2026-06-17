import { useState, useEffect, useCallback } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
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
  BookOpenIcon,
  MoreVerticalIcon,
  PlusIcon,
  Trash2Icon,
  PencilIcon,
  RefreshCwIcon,
  GlobeIcon,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { ipc } from "@/lib/ipc";
import type { BookSource } from "@/types";

export function BookSourcesSettings() {
  const { t } = useI18n();
  const [sources, setSources] = useState<BookSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<BookSource | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);

  // Form state
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
      console.error("Failed to save source:", err);
    }
  }

  async function handleDelete(name: string) {
    try {
      await ipc("novel_source_delete", { name });
      setDeleteConfirm(null);
      loadSources();
    } catch (err) {
      console.error("Failed to delete source:", err);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <BookOpenIcon className="size-5" />
            {t.settings.bookSources}
          </h2>
          <p className="text-sm text-muted-foreground">{t.settings.bookSourcesDesc}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={loadSources} disabled={loading}>
            <RefreshCwIcon data-icon="inline-start" className={loading ? "animate-spin" : ""} />
          </Button>
          <Button size="sm" onClick={openCreateDialog}>
            <PlusIcon data-icon="inline-start" />
            {t.common.create}
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <Spinner className="size-6" />
        </div>
      ) : sources.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpenIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>暂无书源</p>
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {sources.map((source) => (
            <Card key={source.name} className="relative">
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-base flex items-center gap-2">
                      <GlobeIcon className="size-4 shrink-0" />
                      <span>{source.name}</span>
                    </CardTitle>
                    <CardDescription className="mt-1 flex items-center gap-2">
                      <Badge variant={source.disabled ? "secondary" : "default"}>
                        {source.disabled ? "禁用" : "启用"}
                      </Badge>
                      {source.search?.disabled && (
                        <Badge variant="outline">搜索不可用</Badge>
                      )}
                    </CardDescription>
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
                        <span>编辑</span>
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
              </CardHeader>
              <CardContent>
                <p className="line-clamp-1 text-sm text-muted-foreground mb-3">{source.url}</p>
                {source.comment && (
                  <p className="text-xs text-muted-foreground line-clamp-2 mb-3">{source.comment}</p>
                )}
                <div className="flex items-center justify-between">
                  <span className="text-xs text-muted-foreground">
                    {source.search?.disabled ? "无搜索" : "可搜索"}
                  </span>
                  <Switch
                    checked={!source.disabled}
                    onCheckedChange={() => handleToggle(source)}
                    disabled={toggling === source.name}
                  />
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Create/Edit Dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingSource ? "编辑书源" : "新建书源"}</DialogTitle>
            <DialogDescription>
              {editingSource ? "修改书源配置" : "添加新的书源配置"}
            </DialogDescription>
          </DialogHeader>
          <ScrollArea className="max-h-[60vh]">
            <FieldGroup>
              <Field>
                <FieldLabel>名称</FieldLabel>
                <Input value={formName} onChange={(e) => setFormName(e.target.value)} placeholder="书源名称" />
              </Field>
              <Field>
                <FieldLabel>URL</FieldLabel>
                <Input value={formUrl} onChange={(e) => setFormUrl(e.target.value)} placeholder="https://..." />
              </Field>
              <Field>
                <FieldLabel>备注</FieldLabel>
                <Input value={formComment} onChange={(e) => setFormComment(e.target.value)} placeholder="书源备注信息" />
              </Field>
              <Field>
                <div className="flex items-center justify-between">
                  <FieldLabel>启用</FieldLabel>
                  <Switch checked={formEnabled} onCheckedChange={setFormEnabled} />
                </div>
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>搜索 URL</FieldLabel>
                <Input value={formSearchUrl} onChange={(e) => setFormSearchUrl(e.target.value)} placeholder="搜索接口地址" />
              </Field>
              <Field>
                <FieldLabel>搜索方法</FieldLabel>
                <Input value={formSearchMethod} onChange={(e) => setFormSearchMethod(e.target.value)} placeholder="get / post" />
              </Field>
              <Field>
                <FieldLabel>搜索结果选择器</FieldLabel>
                <Textarea value={formSearchResult} onChange={(e) => setFormSearchResult(e.target.value)} placeholder="CSS 选择器" className="min-h-[60px]" />
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>目录选择器</FieldLabel>
                <Input value={formTocItem} onChange={(e) => setFormTocItem(e.target.value)} placeholder="CSS 选择器" />
              </Field>

              <FieldSeparator />

              <Field>
                <FieldLabel>章节标题选择器</FieldLabel>
                <Input value={formChapterTitle} onChange={(e) => setFormChapterTitle(e.target.value)} placeholder="CSS 选择器" />
              </Field>
              <Field>
                <FieldLabel>章节内容选择器</FieldLabel>
                <Input value={formChapterContent} onChange={(e) => setFormChapterContent(e.target.value)} placeholder="CSS 选择器" />
              </Field>
              <Field>
                <FieldLabel>内容过滤规则</FieldLabel>
                <Input value={formChapterFilter} onChange={(e) => setFormChapterFilter(e.target.value)} placeholder="正则表达式" />
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

      {/* Delete Confirmation Dialog */}
      <Dialog open={!!deleteConfirm} onOpenChange={() => setDeleteConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除书源</DialogTitle>
            <DialogDescription>
              确定要删除书源 "{deleteConfirm}" 吗？此操作无法撤销。
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
    </div>
  );
}
