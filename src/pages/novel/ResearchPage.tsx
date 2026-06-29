import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/shared/i18n";
import { parseTags } from "@/shared/utils";
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
import { Badge } from "@/components/ui/badge";
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
  BookmarkIcon,
  PlusIcon,
  Trash2Icon,
  SearchIcon,
  ExternalLinkIcon,
} from "lucide-react";
import { useResearchItems } from "@/features/story/hooks";
import type { ResearchItem, ResearchCategory } from "@/shared/types";

export function ResearchPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { items, loading, create, update, remove } = useResearchItems(activeWorkspaceId);
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

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return items.filter((i) => {
      const matchesCategory = category === "all" || i.category === category;
      const matchesSearch = i.title.toLowerCase().includes(q);
      return matchesCategory && matchesSearch;
    });
  }, [items, category, search]);

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
    if (!formTitle.trim()) return;
    const tags = parseTags(formTags);

    if (isEditing && selected) {
      await update({
        id: selected.id, title: formTitle, content: formContent,
        category: formCategory, tags, source_url: formSourceUrl || null,
      });
    } else {
      await create({
        title: formTitle, content: formContent,
        category: formCategory, tags, source_url: formSourceUrl || null,
      });
    }
    setDialogOpen(false);
  };

  const handleDelete = async (id: string) => {
    await remove(id);
    if (selected?.id === id) setSelected(null);
  };

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <BookmarkIcon />
            {t.research.title}
          </PageTitle>
          <PageDescription>{t.research.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.research.create}
          </Button>
        </PageActions>
      </PageHeader>

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
        <LoadingState label={t.common.loading} />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<BookmarkIcon />} title={t.research.empty} />
      ) : (
        <div className="flex flex-col gap-2">
          {filtered.map((item) => (
            <div
              key={item.id}
              className={`flex items-start gap-3 rounded-lg border p-3 cursor-pointer transition-colors group ${
                selected?.id === item.id ? "border-primary bg-primary/5" : "hover:bg-muted"
              }`}
              onClick={() => openEdit(item)}
            >
              <BookmarkIcon className="size-4 text-muted-foreground shrink-0 mt-0.5" />
              <div className="flex flex-col gap-2 flex-1 min-w-0">
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
                  <p className="text-xs text-muted-foreground line-clamp-2">{item.content}</p>
                )}
                {item.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {item.tags.slice(0, 5).map((tag) => (
                      <Badge key={tag} variant="outline">{tag}</Badge>
                    ))}
                  </div>
                )}
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }}
                className="opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive shrink-0"
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
    </PageContainer>
  );
}
