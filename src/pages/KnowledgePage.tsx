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
  SearchIcon,
  PencilIcon,
  Trash2Icon,
  MoreVerticalIcon,
  TagIcon,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { parseTags } from "@/shared/utils";
import { useKnowledge } from "@/features/knowledge/hooks/useKnowledge";
import type { KnowledgeEntry } from "@/shared/types";

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
          <PageTitle>
            <BookOpenIcon />
            {t.knowledge.title}
          </PageTitle>
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
        <div className="relative flex-1 max-w-sm">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          <Input
            placeholder={t.knowledge.search}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
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
        <Card>
          <CardContent className="p-0">
            <div className="divide-y">
              {entries.map((entry) => (
                <div
                  key={entry.id}
                  className="flex items-start gap-4 px-4 py-3 hover:bg-muted/50 transition-colors"
                >
                  <div className="flex flex-col gap-1.5 flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium truncate">{entry.title}</span>
                      <Badge variant="secondary" className="shrink-0 text-xs">
                        {t.knowledge.categories[entry.category as keyof typeof t.knowledge.categories]}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground line-clamp-2 whitespace-pre-wrap">
                      {entry.content}
                    </p>
                    {entry.tags.length > 0 && (
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <TagIcon className="size-3 text-muted-foreground" />
                        {entry.tags.map((tag) => (
                          <Badge key={tag} variant="outline" className="text-xs">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    )}
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm" className="shrink-0">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openEdit(entry)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        onClick={() => handleDelete(entry.id)}
                        className="text-destructive"
                      >
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
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
