import { useState, useEffect, useCallback } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  BookmarkIcon,
  PlusIcon,
  Trash2Icon,
  SearchIcon,
  ExternalLinkIcon,
} from "lucide-react";
import { ipc } from "@/lib/ipc";
import type { ResearchItem, ResearchCategory } from "@/types";

export function ResearchPage() {
  const { t } = useI18n();
  const { activeWorkspaceId } = useWorkspaceStore();
  const [items, setItems] = useState<ResearchItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [category, setCategory] = useState<ResearchCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<ResearchItem | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

  const [formTitle, setFormTitle] = useState("");
  const [formContent, setFormContent] = useState("");
  const [formCategory, setFormCategory] = useState<ResearchCategory>("note");
  const [formTags, setFormTags] = useState("");
  const [formSourceUrl, setFormSourceUrl] = useState("");

  const loadItems = useCallback(async () => {
    if (!activeWorkspaceId) return;
    setLoading(true);
    try {
      const novels = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novels.find((n) => n.workspace_id === activeWorkspaceId);
      if (!novel) { setItems([]); return; }
      const data = await ipc<ResearchItem[]>("research_item_list", { novelId: novel.id });
      setItems(data);
    } catch {
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, [activeWorkspaceId]);

  useEffect(() => { loadItems(); }, [loadItems]);

  const filtered = items.filter((i) => {
    const matchesCategory = category === "all" || i.category === category;
    const matchesSearch = i.title.toLowerCase().includes(search.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const resetForm = () => {
    setFormTitle(""); setFormContent(""); setFormCategory("note");
    setFormTags(""); setFormSourceUrl("");
  };

  const openCreate = () => { resetForm(); setIsEditing(false); setDialogOpen(true); };

  const openEdit = (item: ResearchItem) => {
    setFormTitle(item.title); setFormContent(item.content);
    setFormCategory(item.category); setFormTags(item.tags.join(", "));
    setFormSourceUrl(item.source_url || "");
    setIsEditing(true); setSelected(item); setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!activeWorkspaceId || !formTitle.trim()) return;
    const novels = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novels.find((n) => n.workspace_id === activeWorkspaceId);
    if (!novel) return;

    const tags = formTags.split(",").map((s) => s.trim()).filter(Boolean);

    if (isEditing && selected) {
      await ipc("research_item_update", {
        id: selected.id, title: formTitle, content: formContent,
        category: formCategory, tags, source_url: formSourceUrl || null,
      });
    } else {
      await ipc("research_item_create", {
        novelId: novel.id, title: formTitle, content: formContent,
        category: formCategory, tags, source_url: formSourceUrl || null,
      });
    }
    setDialogOpen(false);
    await loadItems();
  };

  const handleDelete = async (id: string) => {
    await ipc("research_item_delete", { id });
    if (selected?.id === id) setSelected(null);
    await loadItems();
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BookmarkIcon />
            {t.research.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.research.description}</p>
        </div>
        <Button onClick={openCreate}>
          <PlusIcon data-icon="inline-start" />
          {t.research.create}
        </Button>
      </div>

      <Tabs value={category} onValueChange={(v) => setCategory(v as ResearchCategory | "all")}>
        <TabsList>
          <TabsTrigger value="all">{t.common.search}</TabsTrigger>
          <TabsTrigger value="reference">{t.research.categories.reference}</TabsTrigger>
          <TabsTrigger value="inspiration">{t.research.categories.inspiration}</TabsTrigger>
          <TabsTrigger value="note">{t.research.categories.note}</TabsTrigger>
          <TabsTrigger value="link">{t.research.categories.link}</TabsTrigger>
        </TabsList>
      </Tabs>

      <div className="relative">
        <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t.common.search}
          className="pl-9"
        />
      </div>

      {loading ? (
        <div className="text-center py-8 text-muted-foreground">{t.common.loading}</div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookmarkIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>{t.research.empty}</p>
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((item) => (
            <div
              key={item.id}
              className={`flex items-start gap-3 rounded-lg border p-3 cursor-pointer transition-colors ${
                selected?.id === item.id ? "border-primary bg-primary/5" : "hover:bg-muted"
              }`}
              onClick={() => openEdit(item)}
            >
              <BookmarkIcon className="size-4 text-muted-foreground shrink-0 mt-0.5" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{item.title}</span>
                  <span className="text-[10px] text-muted-foreground">
                    {t.research.categories[item.category]}
                  </span>
                  {item.source_url && (
                    <ExternalLinkIcon className="size-3 text-muted-foreground" />
                  )}
                </div>
                {item.content && (
                  <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{item.content}</p>
                )}
                {item.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-2">
                    {item.tags.slice(0, 5).map((tag) => (
                      <span key={tag} className="text-[10px] bg-muted px-1.5 py-0.5 rounded">{tag}</span>
                    ))}
                  </div>
                )}
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }}
                className="opacity-0 group-hover:opacity-100 hover:text-destructive shrink-0"
              >
                <Trash2Icon className="size-3" />
              </button>
            </div>
          ))}
        </div>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.research.edit : t.research.create}</DialogTitle>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.research.title_label}</FieldLabel>
              <Input value={formTitle} onChange={(e) => setFormTitle(e.target.value)} placeholder={t.research.titlePlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.research.category}</FieldLabel>
              <Select value={formCategory} onValueChange={(v) => setFormCategory(v as ResearchCategory)}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="reference">{t.research.categories.reference}</SelectItem>
                  <SelectItem value="inspiration">{t.research.categories.inspiration}</SelectItem>
                  <SelectItem value="note">{t.research.categories.note}</SelectItem>
                  <SelectItem value="link">{t.research.categories.link}</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field>
              <FieldLabel>{t.research.content}</FieldLabel>
              <Textarea
                value={formContent}
                onChange={(e) => setFormContent(e.target.value)}
                placeholder={t.research.contentPlaceholder}
                className="min-h-[200px]"
              />
            </Field>
            <Field>
              <FieldLabel>{t.research.tags}</FieldLabel>
              <Input value={formTags} onChange={(e) => setFormTags(e.target.value)} placeholder={t.research.tagsPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.research.sourceUrl}</FieldLabel>
              <Input value={formSourceUrl} onChange={(e) => setFormSourceUrl(e.target.value)} placeholder={t.research.sourceUrlPlaceholder} />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formTitle.trim()}>
              {isEditing ? t.research.update : t.research.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
