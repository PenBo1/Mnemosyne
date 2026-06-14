import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
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
import { PlusIcon, MoreVerticalIcon, PencilIcon, Trash2Icon, DatabaseIcon, SearchIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useMemory } from "@/hooks/useMemory";
import type { Memory } from "@/types";

export function MemoryPage() {
  const { t } = useI18n();
  const { memories, filterCategory, setFilterCategory, searchQuery, setSearchQuery, categories, create, update, remove } = useMemory();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingMemory, setEditingMemory] = useState<Memory | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("character");
  const [tagsInput, setTagsInput] = useState("");

  function openCreate() {
    setEditingMemory(null);
    setTitle("");
    setContent("");
    setCategory("character");
    setTagsInput("");
    setDialogOpen(true);
  }

  function openEdit(memory: Memory) {
    setEditingMemory(memory);
    setTitle(memory.title);
    setContent(memory.content);
    setCategory(memory.category);
    setTagsInput(memory.tags.join(", "));
    setDialogOpen(true);
  }

  function handleSave() {
    const tags = tagsInput
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean);

    if (editingMemory) {
      update(editingMemory.id, { title, content, category, tags });
    } else {
      create({ title, content, category, tags });
    }
    setDialogOpen(false);
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between px-6 py-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">{t.memory.title}</h1>
          <p className="text-sm text-muted-foreground">{t.memory.description}</p>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button onClick={openCreate}>
              <PlusIcon data-icon="inline-start" />
              <span>{t.memory.create}</span>
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>{editingMemory ? t.memory.edit : t.memory.create}</DialogTitle>
              <DialogDescription>{t.memory.description}</DialogDescription>
            </DialogHeader>
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="memoryTitle">{t.memory.title_label}</Label>
                <Input id="memoryTitle" value={title} onChange={(e) => setTitle(e.target.value)} placeholder={t.memory.titlePlaceholder} />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="memoryContent">{t.memory.content}</Label>
                <Textarea id="memoryContent" value={content} onChange={(e) => setContent(e.target.value)} placeholder={t.memory.contentPlaceholder} rows={6} />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="memoryCategory">{t.memory.category}</Label>
                  <Select value={category} onValueChange={setCategory}>
                    <SelectTrigger id="memoryCategory">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {categories.map((cat) => (
                        <SelectItem key={cat} value={cat}>
                          {t.memory.categories[cat as keyof typeof t.memory.categories]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="memoryTags">{t.memory.tags}</Label>
                  <Input id="memoryTags" value={tagsInput} onChange={(e) => setTagsInput(e.target.value)} placeholder={t.memory.tagsPlaceholder} />
                </div>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDialogOpen(false)}>
                {t.memory.cancel}
              </Button>
              <Button onClick={handleSave} disabled={!title || !content}>
                {editingMemory ? t.memory.update : t.memory.save}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
      <Separator />
      <ScrollArea className="flex-1">
        <div className="p-6">
          <div className="flex items-center gap-4 mb-6">
            <div className="relative flex-1 max-w-sm">
              <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
              <Input
                placeholder={t.memory.search}
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
                <SelectItem value="all">{t.memory.allCategories}</SelectItem>
                {categories.map((cat) => (
                  <SelectItem key={cat} value={cat}>
                    {t.memory.categories[cat as keyof typeof t.memory.categories]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {memories.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <DatabaseIcon className="size-12 mb-4" />
              <p>{t.memory.empty}</p>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {memories.map((memory) => (
                <Card key={memory.id}>
                  <CardHeader className="pb-3">
                    <div className="flex items-start justify-between">
                      <div className="flex-1 min-w-0">
                        <CardTitle className="truncate text-base">{memory.title}</CardTitle>
                        <CardDescription className="mt-1 flex items-center gap-2">
                          <Badge variant="secondary">
                            {t.memory.categories[memory.category as keyof typeof t.memory.categories]}
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
                          <DropdownMenuItem onClick={() => openEdit(memory)}>
                            <PencilIcon />
                            <span>{t.common.edit}</span>
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => remove(memory.id)} className="text-destructive">
                            <Trash2Icon />
                            <span>{t.common.delete}</span>
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </CardHeader>
                  <CardContent>
                    <p className="line-clamp-3 text-sm text-muted-foreground whitespace-pre-wrap">
                      {memory.content}
                    </p>
                    {memory.tags.length > 0 && (
                      <div className="mt-3 flex flex-wrap gap-1">
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
        </div>
      </ScrollArea>
    </div>
  );
}
