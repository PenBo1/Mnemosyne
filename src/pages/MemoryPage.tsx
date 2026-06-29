import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { EmptyState, LoadingState } from "@/components/shared/state";
import {
  PlusIcon,
  MoreVerticalIcon,
  PencilIcon,
  Trash2Icon,
  DatabaseIcon,
  SearchIcon,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { parseTags } from "@/shared/utils";
import { useWorkspaceStore } from "@/stores/workspace";
import { useMemory } from "@/features/memory/hooks";
import type { MemoryEntry, MemoryType } from "@/shared/types/memory";

export function MemoryPage() {
  const { t } = useI18n();
  const activeBookId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const {
    memories,
    stats,
    loading,
    filterEntryType,
    setFilterEntryType,
    searchQuery,
    setSearchQuery,
    create,
    update,
    remove,
    types,
  } = useMemory(activeBookId);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingMemory, setEditingMemory] = useState<MemoryEntry | null>(null);
  const [content, setContent] = useState("");
  const [entryType, setEntryType] = useState<MemoryType>("fact");
  const [chapterInput, setChapterInput] = useState("");
  const [tagsInput, setTagsInput] = useState("");

  if (!activeBookId) {
    return (
      <PageContainer>
        <EmptyState icon={<DatabaseIcon />} title={t.memory.empty} />
      </PageContainer>
    );
  }

  if (loading && memories.length === 0) {
    return (
      <PageContainer>
        <LoadingState />
      </PageContainer>
    );
  }

  function openCreate() {
    setEditingMemory(null);
    setContent("");
    setEntryType("fact");
    setChapterInput("");
    setTagsInput("");
    setDialogOpen(true);
  }

  function openEdit(memory: MemoryEntry) {
    setEditingMemory(memory);
    setContent(memory.content);
    setEntryType(memory.entry_type);
    setChapterInput(memory.chapter?.toString() ?? "");
    setTagsInput(memory.tags.join(", "));
    setDialogOpen(true);
  }

  async function handleSave() {
    const tags = parseTags(tagsInput);
    const chapterNum = chapterInput.trim() ? Number(chapterInput) : null;
    const chapter = chapterNum != null && !Number.isNaN(chapterNum) ? chapterNum : null;

    if (editingMemory) {
      await update(editingMemory.id, content, tags);
    } else {
      await create({ content, entryType, chapter, tags });
    }
    setDialogOpen(false);
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <DatabaseIcon />
            {t.memory.title}
          </PageTitle>
          <PageDescription>{t.memory.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.memory.create}
          </Button>
        </PageActions>
      </PageHeader>

      {stats && (
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          <span>
            {t.memory.stats.main}: <strong>{stats.main}</strong>
          </span>
          <span>
            {t.memory.stats.archival}: <strong>{stats.archival}</strong>
          </span>
        </div>
      )}

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          <Input
            placeholder={t.memory.search}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select
          value={filterEntryType}
          onValueChange={(v) => setFilterEntryType(v as MemoryType | "all")}
        >
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t.memory.allTypes}</SelectItem>
            {types.map((tp) => (
              <SelectItem key={tp} value={tp}>
                {t.memory.types[tp]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {memories.length === 0 ? (
        <EmptyState icon={<DatabaseIcon />} title={t.memory.empty} />
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {memories.map((memory) => (
            <Card key={memory.id} className="transition-shadow hover:shadow-md">
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-base">
                      {memory.content.split("\n")[0] || memory.content.slice(0, 50)}
                    </CardTitle>
                    <CardDescription className="flex items-center gap-2">
                      <Badge variant="secondary">
                        {t.memory.types[memory.entry_type]}
                      </Badge>
                      {memory.chapter != null && (
                        <Badge variant="outline">Ch.{memory.chapter}</Badge>
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
                      <DropdownMenuItem onClick={() => openEdit(memory)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        onClick={() => remove(memory.id)}
                        className="text-destructive"
                      >
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <p className="line-clamp-3 text-sm text-muted-foreground whitespace-pre-wrap">
                  {memory.content}
                </p>
                {memory.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {memory.tags.map((tag) => (
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

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{editingMemory ? t.memory.edit : t.memory.create}</DialogTitle>
            <DialogDescription>{t.memory.description}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.memory.content}</FieldLabel>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder={t.memory.contentPlaceholder}
                rows={6}
              />
            </Field>
            <div className="grid grid-cols-3 gap-4">
              <Field>
                <FieldLabel>{t.memory.entryType}</FieldLabel>
                <Select value={entryType} onValueChange={(v) => setEntryType(v as MemoryType)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {types.map((tp) => (
                      <SelectItem key={tp} value={tp}>
                        {t.memory.types[tp]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t.memory.chapter}</FieldLabel>
                <Input
                  value={chapterInput}
                  onChange={(e) => setChapterInput(e.target.value)}
                  placeholder={t.memory.chapterPlaceholder}
                  type="number"
                  min={1}
                />
              </Field>
              <Field>
                <FieldLabel>{t.memory.tags}</FieldLabel>
                <Input
                  value={tagsInput}
                  onChange={(e) => setTagsInput(e.target.value)}
                  placeholder={t.memory.tagsPlaceholder}
                />
              </Field>
            </div>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.memory.cancel}
            </Button>
            <Button onClick={handleSave} disabled={!content.trim()}>
              {editingMemory ? t.memory.update : t.memory.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
