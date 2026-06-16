import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
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
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { PlusIcon, MoreVerticalIcon, PencilIcon, Trash2Icon, MessageSquareIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { usePrompts } from "@/hooks/usePrompts";

const CATEGORIES = ["general", "writing", "character", "world", "dialogue", "style"];

export function PromptsSettings() {
  const [filterCategory, setFilterCategory] = useState<string>("all");
  const { prompts, loading, error, create, update, remove } = usePrompts(filterCategory === "all" ? undefined : filterCategory);
  const { t } = useI18n();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingPrompt, setEditingPrompt] = useState<{ id: string; name: string; content: string; category: string } | null>(null);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("general");

  async function handleSave() {
    if (editingPrompt) {
      await update(editingPrompt.id, name, content, category);
    } else {
      await create(name, content, category);
    }
    setDialogOpen(false);
    setEditingPrompt(null);
    setName("");
    setContent("");
    setCategory("general");
  }

  async function handleDelete(id: string) {
    await remove(id);
  }

  function openEdit(prompt: { id: string; name: string; content: string; category: string }) {
    setEditingPrompt(prompt);
    setName(prompt.name);
    setContent(prompt.content);
    setCategory(prompt.category);
    setDialogOpen(true);
  }

  function openCreate() {
    setEditingPrompt(null);
    setName("");
    setContent("");
    setCategory("general");
    setDialogOpen(true);
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <MessageSquareIcon className="size-5" />
            {t.settings.prompts}
          </h2>
          <p className="text-sm text-muted-foreground">{t.settings.promptsDesc}</p>
        </div>
        <div className="flex items-center gap-2">
          <Select value={filterCategory} onValueChange={setFilterCategory}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t.prompts.allCategories}</SelectItem>
              {CATEGORIES.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {t.prompts.categories[cat as keyof typeof t.prompts.categories]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger asChild>
              <Button onClick={openCreate}>
                <PlusIcon data-icon="inline-start" />
                <span>{t.prompts.create}</span>
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{editingPrompt ? t.prompts.edit : t.prompts.create}</DialogTitle>
                <DialogDescription>{t.prompts.contentPlaceholder}</DialogDescription>
              </DialogHeader>
              <FieldGroup>
                <Field>
                  <FieldLabel>{t.prompts.name}</FieldLabel>
                  <Input value={name} onChange={(e) => setName(e.target.value)} placeholder={t.prompts.namePlaceholder} />
                </Field>
                <Field>
                  <FieldLabel>{t.prompts.category}</FieldLabel>
                  <Select value={category} onValueChange={setCategory}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {CATEGORIES.map((cat) => (
                        <SelectItem key={cat} value={cat}>
                          {t.prompts.categories[cat as keyof typeof t.prompts.categories]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                <Field>
                  <FieldLabel>{t.prompts.content}</FieldLabel>
                  <Textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    placeholder={t.prompts.contentPlaceholder}
                    rows={8}
                  />
                </Field>
              </FieldGroup>
              <DialogFooter>
                <Button variant="outline" onClick={() => setDialogOpen(false)}>
                  {t.prompts.cancel}
                </Button>
                <Button onClick={handleSave} disabled={!name || !content}>
                  {editingPrompt ? t.prompts.update : t.prompts.save}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <Spinner className="size-6" />
        </div>
      ) : prompts.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <MessageSquareIcon />
            </EmptyMedia>
            <EmptyTitle>{t.prompts.empty}</EmptyTitle>
            <EmptyDescription>{t.prompts.create}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={openCreate}>
              <PlusIcon data-icon="inline-start" />
              {t.prompts.create}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {prompts.map((prompt) => (
            <Card key={prompt.id}>
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-base">{prompt.name}</CardTitle>
                    <CardDescription className="mt-1">
                      <Badge variant="secondary">{prompt.category}</Badge>
                    </CardDescription>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openEdit(prompt)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleDelete(prompt.id)} className="text-destructive">
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </CardHeader>
              <CardContent>
                <p className="line-clamp-4 text-sm text-muted-foreground whitespace-pre-wrap">
                  {prompt.content}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
