import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useI18n } from "@/locales/i18n";
import { parseTags, cn } from "@/lib/utils";
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
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { SearchInput } from "@/components/shared/search-input";
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
  ExternalLinkIcon,
  LayoutGridIcon,
  ListIcon,
} from "lucide-react";
import { useResearchItems } from "@/features/story/hooks";
import type { ResearchItem, ResearchCategory } from "@/features/story/types";

export function ResearchPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { items, loading, create, update, remove } = useResearchItems(activeWorkspaceId);
  const [category, setCategory] = useState<ResearchCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<ResearchItem | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [viewMode, setViewMode] = useState<"card" | "table">("card");

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

      <div className="flex items-center gap-4">
        <Tabs value={category} onValueChange={(v) => setCategory(v as ResearchCategory | "all")}>
          <TabsList>
            <TabsTrigger value="all">{t.common.search}</TabsTrigger>
            <TabsTrigger value="reference">{t.research.categories.reference}</TabsTrigger>
            <TabsTrigger value="inspiration">{t.research.categories.inspiration}</TabsTrigger>
            <TabsTrigger value="note">{t.research.categories.note}</TabsTrigger>
            <TabsTrigger value="link">{t.research.categories.link}</TabsTrigger>
          </TabsList>
        </Tabs>
        <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as "card" | "table")}>
          <TabsList>
            <TabsTrigger value="card"><LayoutGridIcon className="size-3" /> {t.characters.gridView}</TabsTrigger>
            <TabsTrigger value="table"><ListIcon className="size-3" /> {t.timeline.listView}</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      <SearchInput
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder={t.common.search}
      />

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<BookmarkIcon />} title={t.research.empty} />
      ) : viewMode === "card" ? (
        <div className="flex flex-col gap-2">
          {filtered.map((item) => (
            <Card
              key={item.id}
              onClick={() => openEdit(item)}
              className={cn(
                "cursor-pointer transition-colors group",
                selected?.id === item.id ? "ring-primary bg-primary/5" : "hover:bg-[var(--bg-overlay-l2)]"
              )}
            >
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <BookmarkIcon className="size-4 text-muted-foreground" />
                    <CardTitle className="text-base">{item.title}</CardTitle>
                    <Badge variant="outline" className="text-xs">{t.research.categories[item.category]}</Badge>
                    {item.source_url && <ExternalLinkIcon className="size-3 text-muted-foreground" />}
                  </div>
                  <Button variant="ghost" size="icon-sm" onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }} className="opacity-0 group-hover:opacity-100 text-destructive">
                    <Trash2Icon />
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="flex flex-col gap-2">
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
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t.research.title_label}</TableHead>
                <TableHead>{t.research.category}</TableHead>
                <TableHead>{t.research.tags}</TableHead>
                <TableHead>{t.research.sourceUrl}</TableHead>
                <TableHead className="w-8" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((item) => (
                <TableRow
                  key={item.id}
                  onClick={() => openEdit(item)}
                  className="cursor-pointer"
                >
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-2">
                      <BookmarkIcon className="size-3 text-muted-foreground" />
                      {item.title}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-xs">{t.research.categories[item.category]}</Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {item.tags.slice(0, 3).map((tag) => (
                        <Badge key={tag} variant="outline" className="text-xs">{tag}</Badge>
                      ))}
                    </div>
                  </TableCell>
                  <TableCell>
                    {item.source_url ? (
                      <ExternalLinkIcon className="size-3 text-muted-foreground" />
                    ) : "\u2014"}
                  </TableCell>
                  <TableCell>
                    <Button variant="ghost" size="icon-sm" onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }} className="text-destructive">
                      <Trash2Icon />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.research.edit : t.research.create}</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[70vh]">
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
          </ScrollArea>
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