import { useState, useMemo } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
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
  DialogTrigger,
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
import { BookOpenIcon, PlusIcon, SearchIcon, PencilIcon, Trash2Icon, MoreVerticalIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface KnowledgeEntry {
  id: string;
  title: string;
  content: string;
  category: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

const KNOWLEDGE_CATEGORIES = ["writing", "research", "character", "world", "plot", "style", "reference", "other"] as const;

export function KnowledgePage() {
  const { t } = useI18n();
  const [entries, setEntries] = useState<KnowledgeEntry[]>([]);
  const [filterCategory, setFilterCategory] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<KnowledgeEntry | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("writing");
  const [tagsInput, setTagsInput] = useState("");

  const filteredEntries = useMemo(() => {
    return entries.filter((entry) => {
      const matchesCategory = filterCategory === "all" || entry.category === filterCategory;
      const matchesSearch =
        searchQuery === "" ||
        entry.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        entry.content.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  }, [entries, filterCategory, searchQuery]);

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
    const tags = tagsInput
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean);

    const now = new Date().toISOString();

    if (editingEntry) {
      setEntries((prev) =>
        prev.map((e) =>
          e.id === editingEntry.id ? { ...e, title, content, category, tags, updated_at: now } : e
        )
      );
    } else {
      const newEntry: KnowledgeEntry = {
        id: crypto.randomUUID(),
        title,
        content,
        category,
        tags,
        created_at: now,
        updated_at: now,
      };
      setEntries((prev) => [newEntry, ...prev]);
    }
    setDialogOpen(false);
  }

  function handleDelete(id: string) {
    setEntries((prev) => prev.filter((e) => e.id !== id));
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BookOpenIcon />
            {t.knowledge.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.knowledge.description}</p>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button onClick={openCreate}>
              <PlusIcon data-icon="inline-start" />
              {t.knowledge.create}
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>{editingEntry ? t.knowledge.edit : t.knowledge.create}</DialogTitle>
              <DialogDescription>{t.knowledge.description}</DialogDescription>
            </DialogHeader>
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="knowledgeTitle">{t.knowledge.title_label}</Label>
                <Input
                  id="knowledgeTitle"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={t.knowledge.titlePlaceholder}
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="knowledgeContent">{t.knowledge.content}</Label>
                <Textarea
                  id="knowledgeContent"
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  placeholder={t.knowledge.contentPlaceholder}
                  rows={8}
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="knowledgeCategory">{t.knowledge.category}</Label>
                  <Select value={category} onValueChange={setCategory}>
                    <SelectTrigger id="knowledgeCategory">
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
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="knowledgeTags">{t.knowledge.tags}</Label>
                  <Input
                    id="knowledgeTags"
                    value={tagsInput}
                    onChange={(e) => setTagsInput(e.target.value)}
                    placeholder={t.knowledge.tagsPlaceholder}
                  />
                </div>
              </div>
            </div>
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
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          <Input
            placeholder={t.knowledge.search}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={filterCategory} onValueChange={setFilterCategory}>
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t.knowledge.allCategories}</SelectItem>
            {KNOWLEDGE_CATEGORIES.map((cat) => (
              <SelectItem key={cat} value={cat}>
                {t.knowledge.categories[cat as keyof typeof t.knowledge.categories]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {filteredEntries.length === 0 ? (
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
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {filteredEntries.map((entry) => (
            <Card key={entry.id}>
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-base">{entry.title}</CardTitle>
                    <CardDescription className="mt-1 flex items-center gap-2">
                      <Badge variant="secondary">
                        {t.knowledge.categories[entry.category as keyof typeof t.knowledge.categories]}
                      </Badge>
                    </CardDescription>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openEdit(entry)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleDelete(entry.id)} className="text-destructive">
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </CardHeader>
              <CardContent>
                <p className="line-clamp-3 text-sm text-muted-foreground whitespace-pre-wrap">
                  {entry.content}
                </p>
                {entry.tags.length > 0 && (
                  <div className="mt-3 flex flex-wrap gap-1">
                    {entry.tags.map((tag) => (
                      <Badge key={tag} variant="outline" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
