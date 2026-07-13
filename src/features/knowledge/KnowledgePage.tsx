import { useMemo, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { SearchInput } from "@/components/shared/search-input";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState } from "@/components/shared/state";
import {
  BookOpenIcon,
  PlusIcon,
  PencilIcon,
  Trash2Icon,
  TagIcon,
  CalendarIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { parseTags } from "@/lib/utils";
import { useKnowledge } from "@/features/knowledge/hooks/useKnowledge";
import type { KnowledgeEntry } from "@/features/knowledge/types";

const KNOWLEDGE_CATEGORIES = [
  "writing",
  "research",
  "character",
  "world",
  "plot",
  "style",
  "reference",
  "other",
] as const;

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString();
  } catch {
    return dateStr;
  }
}

export function KnowledgePage() {
  const { t } = useI18n();
  const {
    entries,
    allEntries,
    loading,
    filterCategory,
    setFilterCategory,
    searchQuery,
    setSearchQuery,
    create,
    update,
    remove,
  } = useKnowledge();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<KnowledgeEntry | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("writing");
  const [tagsInput, setTagsInput] = useState("");

  function openCreate() {
    setEditingEntry(null);
    setTitle("");
    setContent("");
    setCategory("writing");
    setTagsInput("");
    setDialogOpen(true);
  }

  function openEdit(entry: KnowledgeEntry) {
    setEditingEntry(entry);
    setTitle(entry.title);
    setContent(entry.content);
    setCategory(entry.category);
    setTagsInput(entry.tags.join(", "));
    setDialogOpen(true);
  }

  function handleSave() {
    const tags = parseTags(tagsInput);
    const params = { title, content, category, tags };

    if (editingEntry) {
      update(editingEntry.id, params);
    } else {
      create(params);
    }
    setDialogOpen(false);
  }

  function handleDelete(id: string) {
    remove(id);
  }

  const categoryCounts = useMemo(
    () =>
      allEntries.reduce(
        (acc, entry) => {
          acc[entry.category] = (acc[entry.category] || 0) + 1;
          return acc;
        },
        {} as Record<string, number>,
      ),
    [allEntries],
  );

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.knowledge.title}</PageTitle>
          <PageDescription>{t.knowledge.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.knowledge.create}
          </Button>
        </PageActions>
      </PageHeader>

      <div className="flex items-center gap-3">
        <SearchInput
          placeholder={t.knowledge.search}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
        <Separator orientation="vertical" className="h-6" />
        <div className="flex items-center gap-1">
          <Button
            variant={filterCategory === "all" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setFilterCategory("all")}
          >
            {t.knowledge.allCategories}
            <Badge variant="outline" className="size-5 justify-center text-xs">
              {allEntries.length}
            </Badge>
          </Button>
          {KNOWLEDGE_CATEGORIES.map((cat) => {
            const count = categoryCounts[cat] || 0;
            if (count === 0) return null;
            return (
              <Button
                key={cat}
                variant={filterCategory === cat ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setFilterCategory(filterCategory === cat ? "all" : cat)}
              >
                {t.knowledge.categories[cat as keyof typeof t.knowledge.categories]}
                <Badge variant="outline" className="size-5 justify-center text-xs">
                  {count}
                </Badge>
              </Button>
            );
          })}
        </div>
      </div>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : entries.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <BookOpenIcon />
            </EmptyMedia>
            <EmptyTitle>{t.knowledge.empty}</EmptyTitle>
            <EmptyDescription>{t.knowledge.description}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={openCreate}>
              <PlusIcon data-icon="inline-start" />
              {t.knowledge.create}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="flex flex-col gap-3">
          {entries.map((entry: KnowledgeEntry) => (
            <Card key={entry.id} className="py-0">
              <CardContent className="divide-y px-0">
                <div className="flex flex-col gap-1 px-4 py-3 transition-colors hover:bg-muted/50">
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-sm font-medium truncate">
                        {entry.title}
                      </span>
                      <Badge variant="secondary" className="text-xs shrink-0">
                        {t.knowledge.categories[entry.category as keyof typeof t.knowledge.categories]}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <Button variant="ghost" size="icon-sm" onClick={() => openEdit(entry)}>
                        <PencilIcon className="size-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onClick={() => handleDelete(entry.id)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2Icon className="size-4" />
                      </Button>
                    </div>
                  </div>
                  <p className="text-sm text-muted-foreground line-clamp-2">
                    {entry.content}
                  </p>
                  <div className="flex items-center justify-between gap-2">
                    {entry.tags.length > 0 ? (
                      <div className="flex items-center gap-1 flex-wrap min-w-0">
                        <TagIcon className="size-3 text-muted-foreground shrink-0" />
                        {entry.tags.slice(0, 5).map((tag) => (
                          <Badge key={tag} variant="outline" className="text-xs">
                            {tag}
                          </Badge>
                        ))}
                        {entry.tags.length > 5 && (
                          <span className="text-xs text-muted-foreground">
                            +{entry.tags.length - 5}
                          </span>
                        )}
                      </div>
                    ) : (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                    <span className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
                      <CalendarIcon className="size-3" />
                      {formatDate(entry.updated_at)}
                    </span>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>
              {editingEntry ? t.knowledge.edit : t.knowledge.create}
            </DialogTitle>
            <DialogDescription>{t.knowledge.description}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.knowledge.title_label}</FieldLabel>
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder={t.knowledge.titlePlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.knowledge.content}</FieldLabel>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder={t.knowledge.contentPlaceholder}
                rows={8}
              />
            </Field>
            <div className="grid grid-cols-2 gap-4">
              <Field>
                <FieldLabel>{t.knowledge.category}</FieldLabel>
                <Select value={category} onValueChange={setCategory}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {KNOWLEDGE_CATEGORIES.map((cat) => (
                      <SelectItem key={cat} value={cat}>
                        {t.knowledge.categories[cat as keyof typeof t.knowledge.categories]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t.knowledge.tags}</FieldLabel>
                <Input
                  value={tagsInput}
                  onChange={(e) => setTagsInput(e.target.value)}
                  placeholder={t.knowledge.tagsPlaceholder}
                />
              </Field>
            </div>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.knowledge.cancel}
            </Button>
            <Button onClick={handleSave} disabled={!title || !content}>
              {editingEntry ? t.knowledge.update : t.knowledge.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}